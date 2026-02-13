use std::{cmp::Ordering, collections::hash_map::DefaultHasher, hash::Hasher};

use crate::collections::{HashMap, HashSet};
use crate::evm::{op, vm::Vm};
use crate::utils::execute_until_function_start;

mod calldata;
use calldata::CallDataImpl;

/// Event selector is a 32-byte keccak256 hash of the event signature
pub type EventSelector = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
enum Label {}

const PROBE_STEP_LIMIT: u16 = 12;
const PROBE_GAS_LIMIT: u32 = 2_500;
const STACK_FINGERPRINT_ELEMS: usize = 10;
const MEMORY_FINGERPRINT_WRITES: usize = 6;
const MEMORY_FINGERPRINT_BYTES: usize = 8;
const MAX_PENDING_STATES: usize = 4_096;
const MAX_VISITED_STATES: usize = 50_000;
const EXEC_ROUNDS: [(u32, u8, u32); 3] = [
    // (gas_limit, max_fork_depth, max_steps_per_state)
    (80_000, 2, 2_000),
    (150_000, 4, 5_000),
    (260_000, 5, 10_000),
];

/// Checks if a 32-byte value looks like a keccak256 hash (event selector).
fn is_plausible_event_hash(val: &[u8; 32]) -> bool {
    if val == &[0u8; 32] {
        return false;
    }
    if val[..6] == [0u8; 6] {
        return false;
    }
    if val[26..] == [0u8; 6] {
        return false;
    }
    let mut zero_run = 0u8;
    let mut ff_run = 0u8;
    for &b in val {
        if b == 0 {
            zero_run += 1;
            if zero_run >= 4 {
                return false;
            }
        } else {
            zero_run = 0;
        }
        if b == 0xff {
            ff_run += 1;
            if ff_run >= 4 {
                return false;
            }
        } else {
            ff_run = 0;
        }
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct StateKey {
    context: usize,
    pc: usize,
    stack_len: usize,
    memory_writes: usize,
    stack_hash: u64,
    memory_hash: u64,
}

fn state_key(vm: &Vm<Label, CallDataImpl>, context: usize) -> StateKey {
    let mut stack_hasher = DefaultHasher::new();
    let stack_start = vm.stack.data.len().saturating_sub(STACK_FINGERPRINT_ELEMS);
    for el in &vm.stack.data[stack_start..] {
        stack_hasher.write(&el.data);
    }

    let mut memory_hasher = DefaultHasher::new();
    for (offset, mem) in vm.memory.data.iter().rev().take(MEMORY_FINGERPRINT_WRITES) {
        memory_hasher.write_u32(*offset);
        memory_hasher.write_usize(mem.data.len());
        let n = std::cmp::min(MEMORY_FINGERPRINT_BYTES, mem.data.len());
        memory_hasher.write(&mem.data[..n]);
        if mem.data.len() > n {
            memory_hasher.write(&mem.data[mem.data.len() - n..]);
        }
    }

    StateKey {
        context,
        pc: vm.pc,
        stack_len: vm.stack.data.len(),
        memory_writes: vm.memory.data.len(),
        stack_hash: stack_hasher.finish(),
        memory_hash: memory_hasher.finish(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeOutcome {
    DeadEnd,
    Terminated,
    Alive,
    HitsLog,
}

impl ProbeOutcome {
    fn score(self) -> u8 {
        match self {
            ProbeOutcome::DeadEnd => 0,
            ProbeOutcome::Terminated => 1,
            ProbeOutcome::Alive => 2,
            ProbeOutcome::HitsLog => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JumpDecision {
    KeepCurrent,
    SwitchOther,
    ForkBoth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct JumpClassify {
    decision: JumpDecision,
    needs_more: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ProbeCacheKey {
    context: usize,
    from_pc: usize,
    to_pc: usize,
    stack_top: [u8; 32],
    stack_len: usize,
}

fn probe_cache_key(vm: &Vm<Label, CallDataImpl>, to_pc: usize, context: usize) -> ProbeCacheKey {
    ProbeCacheKey {
        context,
        from_pc: vm.pc,
        to_pc,
        stack_top: vm.stack.peek().map_or([0u8; 32], |v| v.data),
        stack_len: vm.stack.data.len(),
    }
}

fn is_static_dead_end(code: &[u8], pc: usize) -> bool {
    if pc >= code.len() {
        return true;
    }

    let mut cur = pc;
    for _ in 0..10 {
        if cur >= code.len() {
            return false;
        }

        let op = code[cur];
        match op {
            op::REVERT | op::INVALID => return true,

            // Common prelude before revert branch bodies.
            op::JUMPDEST
            | op::PUSH0..=op::PUSH32
            | op::DUP1..=op::DUP16
            | op::SWAP1..=op::SWAP16
            | op::POP => {
                cur += op::info(op).size;
            }

            _ => return false,
        }
    }

    false
}

fn probe_branch_cached(
    vm: &Vm<Label, CallDataImpl>,
    start_pc: usize,
    step_limit: u16,
    gas_limit: u32,
    context: usize,
    cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
) -> ProbeOutcome {
    let key = probe_cache_key(vm, start_pc, context);
    if let Some(outcome) = cache.get(&key) {
        return *outcome;
    }

    let mut branch = vm.fork();
    branch.pc = start_pc;
    let outcome = probe_branch(branch, start_pc, step_limit, gas_limit);
    cache.insert(key, outcome);
    outcome
}

fn probe_branch(
    mut vm: Vm<Label, CallDataImpl>,
    start_pc: usize,
    step_limit: u16,
    gas_limit: u32,
) -> ProbeOutcome {
    if start_pc >= vm.code.len() {
        return ProbeOutcome::DeadEnd;
    }

    vm.pc = start_pc;
    let mut gas_used = 0u32;

    for _ in 0..step_limit {
        if vm.stopped {
            return ProbeOutcome::Terminated;
        }

        let ret = match vm.step() {
            Ok(v) => v,
            Err(_) => return ProbeOutcome::DeadEnd,
        };

        gas_used = gas_used.saturating_add(ret.gas_used);
        if gas_used > gas_limit {
            return ProbeOutcome::Alive;
        }

        match ret.op {
            op::LOG1..=op::LOG4 => return ProbeOutcome::HitsLog,
            op::REVERT | op::INVALID => return ProbeOutcome::DeadEnd,
            op::STOP | op::RETURN | op::SELFDESTRUCT => return ProbeOutcome::Terminated,
            _ => {}
        }
    }

    if vm.stopped {
        ProbeOutcome::Terminated
    } else {
        ProbeOutcome::Alive
    }
}

fn classify_jump(
    vm: &Vm<Label, CallDataImpl>,
    context: usize,
    other_pc: usize,
    can_fork: bool,
    probe_steps: u16,
    probe_gas: u32,
    probe_cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
) -> JumpClassify {
    if other_pc == vm.pc {
        return JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        };
    }

    // Solidity require() revert branches are often statically obvious:
    // JUMPDEST -> PUSH* ... -> REVERT/INVALID.
    let other_static_dead = is_static_dead_end(vm.code, other_pc);
    if other_static_dead {
        return JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        };
    }

    let current_static_dead = is_static_dead_end(vm.code, vm.pc);
    if current_static_dead {
        return JumpClassify {
            decision: JumpDecision::SwitchOther,
            needs_more: false,
        };
    }

    if can_fork {
        return JumpClassify {
            decision: JumpDecision::ForkBoth,
            needs_more: false,
        };
    }

    let other = probe_branch_cached(vm, other_pc, probe_steps, probe_gas, context, probe_cache);
    if other == ProbeOutcome::DeadEnd {
        return JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        };
    }

    let current = probe_branch_cached(vm, vm.pc, probe_steps, probe_gas, context, probe_cache);
    if current == ProbeOutcome::DeadEnd {
        return JumpClassify {
            decision: JumpDecision::SwitchOther,
            needs_more: false,
        };
    }

    match other.score().cmp(&current.score()) {
        Ordering::Greater => JumpClassify {
            decision: JumpDecision::SwitchOther,
            needs_more: false,
        },
        Ordering::Less => JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        },
        Ordering::Equal => JumpClassify {
            decision: JumpDecision::KeepCurrent,
            // Only keep escalating when both branches look equally open-ended.
            needs_more: current == ProbeOutcome::Alive,
        },
    }
}

fn collect_event(
    events: &mut Vec<EventSelector>,
    seen: &mut HashSet<EventSelector>,
    topic0: EventSelector,
) {
    if is_plausible_event_hash(&topic0) && seen.insert(topic0) {
        events.push(topic0);
    }
}

struct BatchState<'a> {
    idx: usize,
    context: usize,
    vm: Vm<'a, Label, CallDataImpl>,
    gas_used: u32,
    depth: u8,
    steps: u32,
}

fn execute_paths<'a>(
    start_vm: Vm<'a, Label, CallDataImpl>,
    initial_gas: u32,
    events: &mut Vec<EventSelector>,
    seen: &mut HashSet<EventSelector>,
    gas_limit: u32,
    max_depth: u8,
    max_steps: u32,
) -> bool {
    let needs_more = execute_paths_batch(
        vec![BatchState {
            idx: 0,
            context: 0,
            vm: start_vm,
            gas_used: initial_gas,
            depth: 0,
            steps: 0,
        }],
        1,
        events,
        seen,
        gas_limit,
        max_depth,
        max_steps,
    );
    needs_more.into_iter().next().unwrap_or(false)
}

fn execute_paths_batch<'a>(
    initial_states: Vec<BatchState<'a>>,
    states_count: usize,
    events: &mut Vec<EventSelector>,
    seen: &mut HashSet<EventSelector>,
    gas_limit: u32,
    max_depth: u8,
    max_steps: u32,
) -> Vec<bool> {
    let mut needs_more = vec![false; states_count];
    if initial_states.is_empty() {
        return needs_more;
    }

    let mut queue = initial_states;
    queue.retain(|s| {
        if s.gas_used > gas_limit {
            needs_more[s.idx] = true;
            false
        } else {
            true
        }
    });

    if queue.is_empty() {
        return needs_more;
    }

    let mut visited: HashSet<StateKey> = HashSet::default();
    let mut probe_cache: HashMap<ProbeCacheKey, ProbeOutcome> = HashMap::default();
    while let Some(state) = queue.pop() {
        let idx = state.idx;
        let context = state.context;
        let mut vm = state.vm;
        let mut gas_used = state.gas_used;
        let depth = state.depth;
        let mut steps = state.steps;

        while !vm.stopped {
            if gas_used >= gas_limit || steps >= max_steps {
                needs_more[idx] = true;
                break;
            }

            let step_pc = vm.pc;
            let ret = match vm.step() {
                Ok(v) => v,
                Err(_) => break,
            };

            gas_used = gas_used.saturating_add(ret.gas_used);
            steps += 1;

            if gas_used > gas_limit {
                break;
            }

            match ret.op {
                op::LOG1..=op::LOG4 => collect_event(events, seen, ret.args[0].data),
                op::JUMPI => {
                    if visited.len() >= MAX_VISITED_STATES {
                        needs_more[idx] = true;
                        break;
                    }
                    if !visited.insert(state_key(&vm, context)) {
                        break;
                    }

                    let cond_zero = ret.args[1].data == [0u8; 32];
                    let other_pc = if cond_zero {
                        usize::try_from(&ret.args[0]).ok()
                    } else {
                        step_pc.checked_add(1)
                    };

                    let Some(other_pc) = other_pc else {
                        continue;
                    };

                    let other_is_valid = if cond_zero {
                        other_pc < vm.code.len() && vm.code[other_pc] == op::JUMPDEST
                    } else {
                        other_pc < vm.code.len()
                    };

                    if !other_is_valid {
                        continue;
                    }

                    let probe_gas = gas_limit.saturating_sub(gas_used).min(PROBE_GAS_LIMIT);
                    let can_fork = depth < max_depth && queue.len() < MAX_PENDING_STATES;
                    let jump = classify_jump(
                        &vm,
                        context,
                        other_pc,
                        can_fork,
                        PROBE_STEP_LIMIT,
                        probe_gas,
                        &mut probe_cache,
                    );
                    if jump.needs_more {
                        needs_more[idx] = true;
                    }

                    match jump.decision {
                        JumpDecision::KeepCurrent => {}
                        JumpDecision::SwitchOther => vm.pc = other_pc,
                        JumpDecision::ForkBoth => {
                            if can_fork {
                                let mut forked = vm.fork();
                                forked.pc = other_pc;
                                queue.push(BatchState {
                                    idx,
                                    context,
                                    vm: forked,
                                    gas_used,
                                    depth: depth + 1,
                                    steps,
                                });
                            }
                        }
                    }
                }
                op::JUMP => {
                    if visited.len() >= MAX_VISITED_STATES {
                        needs_more[idx] = true;
                        break;
                    }
                    if !visited.insert(state_key(&vm, context)) {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    needs_more
}

fn execute_from_entry(
    code: &[u8],
    calldata: &CallDataImpl,
    events: &mut Vec<EventSelector>,
    seen: &mut HashSet<EventSelector>,
    gas_limit: u32,
    max_depth: u8,
    max_steps: u32,
) -> bool {
    let vm = Vm::new(code, calldata);
    execute_paths(vm, 0, events, seen, gas_limit, max_depth, max_steps)
}

/// Extracts all event selectors from contract bytecode.
pub fn contract_events(code: &[u8]) -> Vec<EventSelector> {
    if code.is_empty() {
        return Vec::new();
    }

    let (selectors, _) = crate::selectors::function_selectors(code, 0);
    let mut events = Vec::<EventSelector>::new();
    let mut seen = HashSet::<EventSelector>::default();
    let mut stable_rounds = 0u8;

    if selectors.is_empty() {
        let calldata = CallDataImpl { selector: [0; 4] };
        let mut pending = true;
        for &(gas_limit, max_depth, max_steps) in &EXEC_ROUNDS {
            if !pending {
                break;
            }
            let before = seen.len();
            pending = execute_from_entry(
                code,
                &calldata,
                &mut events,
                &mut seen,
                gas_limit,
                max_depth,
                max_steps,
            );
            if seen.len() == before {
                stable_rounds += 1;
                if stable_rounds >= 2 {
                    break;
                }
            } else {
                stable_rounds = 0;
            }
        }
    } else {
        let mut pending: Vec<([u8; 4], usize)> = selectors
            .iter()
            .map(|(selector, offset)| (*selector, *offset))
            .collect();

        for &(gas_limit, max_depth, max_steps) in &EXEC_ROUNDS {
            if pending.is_empty() {
                break;
            }
            let before = seen.len();

            let calldatas: Vec<CallDataImpl> = pending
                .iter()
                .map(|(selector, _)| CallDataImpl {
                    selector: *selector,
                })
                .collect();

            let mut initial_states = Vec::with_capacity(pending.len());
            let mut round_needs_more = vec![false; pending.len()];

            for (idx, ((_, offset), calldata)) in pending.iter().zip(calldatas.iter()).enumerate() {
                if *offset < code.len() && code[*offset] == op::JUMPDEST {
                    let mut vm = Vm::new(code, calldata);
                    vm.pc = *offset;
                    initial_states.push(BatchState {
                        idx,
                        context: *offset,
                        vm,
                        gas_used: 0,
                        depth: 0,
                        steps: 0,
                    });
                } else {
                    let mut vm = Vm::new(code, calldata);
                    if let Some(initial_gas) = execute_until_function_start(&mut vm, gas_limit) {
                        initial_states.push(BatchState {
                            idx,
                            context: *offset,
                            vm,
                            gas_used: initial_gas,
                            depth: 0,
                            steps: 0,
                        });
                    } else {
                        round_needs_more[idx] = true;
                    }
                }
            }

            let batch_needs_more = execute_paths_batch(
                initial_states,
                pending.len(),
                &mut events,
                &mut seen,
                gas_limit,
                max_depth,
                max_steps,
            );

            for (dst, src) in round_needs_more.iter_mut().zip(batch_needs_more.iter()) {
                *dst |= *src;
            }

            pending = pending
                .into_iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    if round_needs_more[idx] {
                        Some(item)
                    } else {
                        None
                    }
                })
                .collect();

            if seen.len() == before {
                stable_rounds += 1;
                if stable_rounds >= 2 {
                    break;
                }
            } else {
                stable_rounds = 0;
            }
        }
    }

    events.sort_unstable();
    events
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::evm::op;

    fn append_log1(code: &mut Vec<u8>, selector: [u8; 32]) {
        code.push(op::PUSH32);
        code.extend_from_slice(&selector);
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1]);
    }

    fn append_single_selector_dispatch(code: &mut Vec<u8>, selector: [u8; 4]) -> usize {
        code.extend_from_slice(&[
            op::PUSH1,
            0x00,
            op::CALLDATALOAD,
            op::PUSH1,
            0xE0,
            op::SHR,
            op::PUSH4,
        ]);
        code.extend_from_slice(&selector);
        code.push(op::EQ);
        code.extend_from_slice(&[op::PUSH1, 0x00]);
        let entry_patch = code.len() - 1;
        code.push(op::JUMPI);
        code.push(op::STOP);
        entry_patch
    }

    #[test]
    fn test_simple_log1() {
        let selector = [0xab; 32];
        let mut code = Vec::new();
        append_log1(&mut code, selector);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![selector]);
    }

    #[test]
    fn test_require_guarded_event() {
        let function_selector = [0xaa, 0xbb, 0xcc, 0xdd];
        let event_selector = [0x42; 32];

        let mut code = Vec::new();
        let entry_patch = append_single_selector_dispatch(&mut code, function_selector);

        let function_entry = code.len();
        code[entry_patch] = u8::try_from(function_entry).unwrap();
        code.push(op::JUMPDEST);

        // Emulate a require guard:
        // if (!cond) revert(); else emit LOG1(topic0)
        code.extend_from_slice(&[op::PUSH1, 0x00]); // cond = 0
        code.extend_from_slice(&[op::PUSH1, 0x00]); // destination (patched below)
        let emit_patch = code.len() - 1;
        code.extend_from_slice(&[op::JUMPI, op::PUSH1, 0x00, op::PUSH1, 0x00, op::REVERT]);
        let emit_pc = code.len();
        code[emit_patch] = u8::try_from(emit_pc).unwrap();

        code.push(op::JUMPDEST);
        append_log1(&mut code, event_selector);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert_eq!(events, vec![event_selector]);
    }

    #[test]
    fn test_forks_when_both_branches_are_alive() {
        let function_selector = [0xaa, 0xbb, 0xcc, 0xdd];
        let event_true = [0x11; 32];
        let event_false = [0x22; 32];

        let mut code = Vec::new();
        let entry_patch = append_single_selector_dispatch(&mut code, function_selector);

        let function_entry = code.len();
        code[entry_patch] = u8::try_from(function_entry).unwrap();
        code.push(op::JUMPDEST);

        // Always-false condition. VM takes fallthrough branch, but both branches emit,
        // so branch classifier should fork and collect both events.
        code.extend_from_slice(&[op::PUSH1, 0x00]); // cond = 0
        code.extend_from_slice(&[op::PUSH1, 0x00]); // true destination (patched below)
        let true_patch = code.len() - 1;
        code.push(op::JUMPI);

        code.push(op::JUMPDEST);
        append_log1(&mut code, event_false);
        code.push(op::STOP);

        let true_pc = code.len();
        code[true_patch] = u8::try_from(true_pc).unwrap();

        code.push(op::JUMPDEST);
        append_log1(&mut code, event_true);
        code.push(op::STOP);

        let events = contract_events(&code);
        let found: BTreeSet<_> = events.into_iter().collect();
        let expected: BTreeSet<_> = [event_true, event_false].into_iter().collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn test_no_events() {
        let code = alloy_primitives::hex::decode("6080604052348015600e575f80fd5b50").unwrap();
        let events = contract_events(&code);
        assert!(events.is_empty());
    }

    #[test]
    fn test_push32_no_log() {
        let mut code = Vec::new();
        code.push(op::PUSH32);
        code.extend_from_slice(&[0xab; 32]);
        code.push(op::POP);
        code.push(op::STOP);

        let events = contract_events(&code);
        assert!(events.is_empty());
    }
}
