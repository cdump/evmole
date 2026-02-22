use std::{
    cmp::Ordering, collections::BTreeMap, collections::VecDeque,
    collections::hash_map::DefaultHasher, hash::Hasher,
};

use crate::Selector;
use crate::collections::{HashMap, HashSet};
use crate::control_flow_graph::{
    Block, BlockType, INVALID_JUMP_START, basic_blocks, control_flow_graph,
};
use crate::evm::{code_iterator::iterate_code, element::Element, memory::LabeledVec, op, vm::Vm};
use crate::utils::execute_until_function_start;

mod bottom_up;
mod calldata;
use calldata::CallDataImpl;

/// Event selector is a 32-byte keccak256 hash of the event signature
pub type EventSelector = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
enum Label {
    ExternalCallResult(usize),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventExtractionStats {
    pub selectors_total: u64,
    pub selectors_after_mutability_prune: u64,
    pub selectors_pruned_view_or_pure: u64,
    pub jump_classify_cache_hits: u64,
    pub jump_classify_cache_misses: u64,
    pub entry_state_cache_hits: u64,
    pub entry_state_cache_misses: u64,
    pub jump_classify_can_fork_true: u64,
    pub jump_classify_can_fork_false: u64,
    pub probe_cache_hits: u64,
    pub probe_cache_misses: u64,
    pub static_dead_other_prunes: u64,
    pub static_dead_current_prunes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventExecutionProfile {
    pub states_pushed: u64,
    pub states_popped: u64,
    pub queue_peak: usize,
    pub state_limit_breaks: u64,
    pub visited_cap_hits: u64,

    pub jump_total: u64,
    pub jump_visited_breaks: u64,

    pub jumpi_total: u64,
    pub jumpi_visited_breaks: u64,
    pub jumpi_invalid_other_pc: u64,
    pub jumpi_unreachable_both: u64,
    pub jumpi_unreachable_current: u64,
    pub jumpi_unreachable_other: u64,
    pub jumpi_fork_throttled: u64,
    pub jumpi_fork_deduped: u64,
    pub jumpi_decision_keep: u64,
    pub jumpi_decision_switch: u64,
    pub jumpi_decision_fork: u64,

    pub context_start_by_pc: BTreeMap<usize, u64>,
    pub jumpi_by_pc: BTreeMap<usize, u64>,
    pub jumpi_can_fork_true_by_pc: BTreeMap<usize, u64>,
    pub jumpi_can_fork_false_by_pc: BTreeMap<usize, u64>,
    pub jumpi_cache_hit_by_pc: BTreeMap<usize, u64>,
    pub jumpi_cache_miss_by_pc: BTreeMap<usize, u64>,
    pub jumpi_decision_keep_by_pc: BTreeMap<usize, u64>,
    pub jumpi_decision_switch_by_pc: BTreeMap<usize, u64>,
    pub jumpi_decision_fork_by_pc: BTreeMap<usize, u64>,
    pub jumpi_invalid_other_pc_by_pc: BTreeMap<usize, u64>,
    pub jumpi_unreachable_both_by_pc: BTreeMap<usize, u64>,
    pub jumpi_unreachable_current_by_pc: BTreeMap<usize, u64>,
    pub jumpi_unreachable_other_by_pc: BTreeMap<usize, u64>,
    pub jumpi_fork_throttled_by_pc: BTreeMap<usize, u64>,
    pub jumpi_fork_deduped_by_pc: BTreeMap<usize, u64>,
    pub jumpi_visited_breaks_by_pc: BTreeMap<usize, u64>,
}

fn bump_pc(map: &mut BTreeMap<usize, u64>, pc: usize) {
    *map.entry(pc).or_insert(0) += 1;
}

const PROBE_STEP_LIMIT: u16 = 12;
const PROBE_GAS_LIMIT: u32 = 2_500;
const STATIC_DEAD_END_SCAN_STEPS: u8 = 16;
const STATIC_DEAD_END_MAX_FOLLOW: u8 = 4;
const DIRECT_ENTRY_SCAN_STEPS: u8 = 24;
const STATIC_EVENT_SCAN_WINDOW_METRIC: u8 = 24;
const STATIC_EVENT_SCAN_WINDOW_SUPPLEMENT: u8 = 192;
const STACK_FINGERPRINT_ELEMS: usize = 10;
const MEMORY_FINGERPRINT_WRITES: usize = 6;
const MEMORY_FINGERPRINT_BYTES: usize = 8;
const MAX_PENDING_STATES: usize = 4_096;
const MAX_VISITED_STATES: usize = 50_000;
const STARVATION_LOCAL_UPLIFT_MAX_VISITED_STATES: usize = 120_000;
const STARVATION_LOCAL_UPLIFT_MAX_PENDING_SELECTORS: usize = 32;
const STARVATION_LOCAL_UPLIFT_MAX_CHUNKS: usize = 2;
const MAX_JUMPI_FORKS_PER_CONTEXT_PC: u16 = 128;
const MAX_JUMPI_FORKS_PER_EQUIV_KEY: u8 = 2;
const MAX_CALL_FAIL_FORKS_PER_CONTEXT_PC: u8 = 1;
const DYNAMIC_JUMP_HOT_FANOUT_MIN: usize = 24;
const DYNAMIC_JUMP_EXTRA_TARGETS_PER_HIT: usize = 2;
const MAX_DYNAMIC_JUMP_EXTRA_FORKS_PER_SITE: u16 = 16;
const MAX_DYNAMIC_JUMP_EXTRA_FORKS_PER_TARGET: u8 = 1;
const MAX_JUMP_CLASSIFY_CACHE: usize = 100_000;
const MAX_ENTRY_STATE_CACHE: usize = 16_384;
const FAST_EXEC_ROUNDS: [(u32, u8, u32); 3] = [
    // (gas_limit, max_fork_depth, max_steps_per_state)
    (80_000, 2, 2_000),
    (150_000, 4, 5_000),
    (260_000, 5, 10_000),
];
const RECALL_EXEC_ROUNDS: [(u32, u8, u32); 2] = [
    // Fallback rounds for unresolved paths (recall-first).
    (520_000, 7, 30_000),
    (1_200_000, 10, 120_000),
];
const RECALL_EXEC_ROUNDS_PARTIAL: [(u32, u8, u32); 1] = [
    // Lighter recall when we already found at least one event.
    (320_000, 6, 20_000),
];

fn should_run_starvation_local_uplift(
    _selectors_total: usize,
    pending_selectors: usize,
    starved_pending: usize,
) -> bool {
    pending_selectors > 0 && starved_pending > 0
}

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
    to_pc: usize,
    stack_top: [u8; 32],
    stack_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ForkDedupeKey {
    context: usize,
    step_pc: usize,
    other_pc: usize,
    stack_top: [u8; 32],
    stack_len_bucket: u8,
}

fn stack_len_bucket(stack_len: usize) -> u8 {
    (stack_len / 4).min(u8::MAX as usize) as u8
}

fn fork_dedupe_key(
    vm: &Vm<Label, CallDataImpl>,
    context: usize,
    step_pc: usize,
    other_pc: usize,
) -> ForkDedupeKey {
    ForkDedupeKey {
        context,
        step_pc,
        other_pc,
        stack_top: vm.stack.peek().map_or([0u8; 32], |v| v.data),
        stack_len_bucket: stack_len_bucket(vm.stack.data.len()),
    }
}

fn probe_cache_key(vm: &Vm<Label, CallDataImpl>, to_pc: usize, context: usize) -> ProbeCacheKey {
    ProbeCacheKey {
        context,
        to_pc,
        stack_top: vm.stack.peek().map_or([0u8; 32], |v| v.data),
        stack_len: vm.stack.data.len(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct JumpClassifyCacheKey {
    state: StateKey,
    step_pc: usize,
    other_pc: usize,
    prefer_other_on_tie: bool,
    probe_steps: u16,
    probe_gas: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct EntryStateCacheKey {
    selector: [u8; 4],
    offset: usize,
}

#[derive(Clone, Debug)]
struct EntryStateSnapshot {
    pc: usize,
    gas_used: u32,
    stack: Vec<[u8; 32]>,
    memory: Vec<(u32, Vec<u8>)>,
}

fn snapshot_entry_state(vm: &Vm<Label, CallDataImpl>, gas_used: u32) -> EntryStateSnapshot {
    EntryStateSnapshot {
        pc: vm.pc,
        gas_used,
        stack: vm.stack.data.iter().map(|el| el.data).collect(),
        memory: vm
            .memory
            .data
            .iter()
            .map(|(offset, chunk)| (*offset, chunk.data.clone()))
            .collect(),
    }
}

fn restore_entry_state<'a>(
    code: &'a [u8],
    calldata: &'a CallDataImpl,
    snapshot: &EntryStateSnapshot,
) -> Vm<'a, Label, CallDataImpl> {
    let mut vm = Vm::new(code, calldata);
    vm.pc = snapshot.pc;
    vm.stack.data = snapshot
        .stack
        .iter()
        .map(|data| Element {
            data: *data,
            label: None,
        })
        .collect();
    vm.memory.data = snapshot
        .memory
        .iter()
        .map(|(offset, data)| {
            (
                *offset,
                LabeledVec {
                    data: data.clone(),
                    label: None,
                },
            )
        })
        .collect();
    vm.stopped = vm.pc >= vm.code.len();
    vm
}

fn is_static_dead_end(code: &[u8], pc: usize) -> bool {
    fn push_target(code: &[u8], pc: usize, opv: u8) -> Option<(usize, usize)> {
        let n = (opv - op::PUSH0) as usize;
        let imm_start = pc.checked_add(1)?;
        let imm_end = imm_start.checked_add(n)?;
        let imm = code.get(imm_start..imm_end)?;

        let mut target = 0usize;
        for &b in imm {
            target = target.checked_mul(256)?.checked_add(b as usize)?;
        }
        Some((imm_end, target))
    }

    fn inner(code: &[u8], pc: usize, depth: u8, seen: &mut HashSet<usize>) -> bool {
        if pc >= code.len() || depth > STATIC_DEAD_END_MAX_FOLLOW || !seen.insert(pc) {
            return false;
        }

        let mut cur = pc;
        for _ in 0..STATIC_DEAD_END_SCAN_STEPS {
            if cur >= code.len() {
                return false;
            }

            let opv = code[cur];
            match opv {
                op::REVERT | op::INVALID => return true,
                op::STOP | op::RETURN | op::SELFDESTRUCT => return false,

                op::PUSH1..=op::PUSH32 => {
                    let Some((next_pc, target)) = push_target(code, cur, opv) else {
                        return false;
                    };

                    // Follow shared revert handler pattern:
                    // PUSH <dst>; JUMP
                    if next_pc < code.len() && code[next_pc] == op::JUMP {
                        if target < code.len() && code[target] == op::JUMPDEST {
                            return inner(code, target, depth + 1, seen);
                        }
                        return false;
                    }

                    cur = next_pc;
                }

                // Common harmless ops seen in revert/error encoding paths.
                op::JUMPDEST
                | op::PUSH0
                | op::DUP1..=op::DUP16
                | op::SWAP1..=op::SWAP16
                | op::POP
                | op::MSTORE
                | op::MSTORE8
                | op::CALLDATASIZE
                | op::ADD
                | op::SUB
                | op::AND
                | op::OR
                | op::SHL
                | op::SHR
                | op::NOT
                | op::ISZERO => {
                    cur += op::info(opv).size;
                }

                _ => return false,
            }
        }

        false
    }

    let mut seen = HashSet::default();
    inner(code, pc, 0, &mut seen)
}

fn is_static_dead_end_cached(code: &[u8], pc: usize, cache: &mut HashMap<usize, bool>) -> bool {
    if let Some(v) = cache.get(&pc) {
        return *v;
    }
    let v = is_static_dead_end(code, pc);
    cache.insert(pc, v);
    v
}

fn push_immediate_target(code: &[u8], push_pc: usize, push_op: u8) -> Option<usize> {
    if !(op::PUSH0..=op::PUSH32).contains(&push_op) {
        return None;
    }

    let n = (push_op - op::PUSH0) as usize;
    let imm_start = push_pc.checked_add(1)?;
    let imm_end = imm_start.checked_add(n)?;
    let imm = code.get(imm_start..imm_end)?;

    let mut target = 0usize;
    for &b in imm {
        target = target.checked_mul(256)?.checked_add(b as usize)?;
    }
    Some(target)
}

fn compute_may_reach_log(code: &[u8]) -> Vec<bool> {
    let mut fallthrough_preds: Vec<Vec<usize>> = vec![Vec::new(); code.len()];
    let mut jump_target_preds: Vec<Vec<usize>> = vec![Vec::new(); code.len()];
    let mut unresolved_dynamic_jump_sources: Vec<usize> = Vec::new();
    let mut static_jump_candidates: Vec<(usize, usize)> = Vec::new();
    let mut is_jumpdest = vec![false; code.len()];
    let mut is_log = vec![false; code.len()];
    let mut instr_pcs = Vec::new();
    let mut prev_instr: Option<(usize, u8)> = None;

    for (pc, cop) in iterate_code(code, 0, None) {
        instr_pcs.push(pc);
        let opv = cop.op;
        if opv == op::JUMPDEST {
            is_jumpdest[pc] = true;
        }
        if (op::LOG1..=op::LOG4).contains(&opv) {
            is_log[pc] = true;
        }

        let next_pc = pc + cop.opi.size;
        match opv {
            op::STOP | op::RETURN | op::REVERT | op::INVALID | op::SELFDESTRUCT => {}
            op::JUMP => {
                if let Some((prev_pc, prev_op)) = prev_instr {
                    if let Some(target) = push_immediate_target(code, prev_pc, prev_op) {
                        static_jump_candidates.push((pc, target));
                    } else {
                        unresolved_dynamic_jump_sources.push(pc);
                    }
                } else {
                    unresolved_dynamic_jump_sources.push(pc);
                }
            }
            op::JUMPI => {
                if let Some((prev_pc, prev_op)) = prev_instr {
                    if let Some(target) = push_immediate_target(code, prev_pc, prev_op) {
                        static_jump_candidates.push((pc, target));
                    } else {
                        unresolved_dynamic_jump_sources.push(pc);
                    }
                } else {
                    unresolved_dynamic_jump_sources.push(pc);
                }
                if next_pc < code.len() {
                    fallthrough_preds[next_pc].push(pc);
                }
            }
            _ => {
                if next_pc < code.len() {
                    fallthrough_preds[next_pc].push(pc);
                }
            }
        }

        prev_instr = Some((pc, opv));
    }

    for (src, target) in static_jump_candidates {
        if target < code.len() && is_jumpdest[target] {
            jump_target_preds[target].push(src);
        } else {
            unresolved_dynamic_jump_sources.push(src);
        }
    }

    let mut reachable = vec![false; code.len()];
    let mut queue = Vec::new();
    for pc in instr_pcs.iter().copied() {
        if is_log[pc] {
            reachable[pc] = true;
            queue.push(pc);
        }
    }

    let mut dynamic_sources_enqueued = false;
    while let Some(pc) = queue.pop() {
        for &pred in &fallthrough_preds[pc] {
            if !reachable[pred] {
                reachable[pred] = true;
                queue.push(pred);
            }
        }
        for &pred in &jump_target_preds[pc] {
            if !reachable[pred] {
                reachable[pred] = true;
                queue.push(pred);
            }
        }

        if is_jumpdest[pc]
            && !dynamic_sources_enqueued
            && !unresolved_dynamic_jump_sources.is_empty()
        {
            dynamic_sources_enqueued = true;
            for &src in &unresolved_dynamic_jump_sources {
                if !reachable[src] {
                    reachable[src] = true;
                    queue.push(src);
                }
            }
        }
    }

    reachable
}

fn is_safe_direct_function_entry(code: &[u8], offset: usize) -> bool {
    if offset >= code.len() || code[offset] != op::JUMPDEST {
        return false;
    }

    let mut pc = offset + 1;
    let mut stack_depth = 0i32;

    for _ in 0..DIRECT_ENTRY_SCAN_STEPS {
        if pc >= code.len() {
            return true;
        }

        let opv = code[pc];
        let opi = op::info(opv);
        if !opi.known {
            return false;
        }

        let in_need = i32::try_from(opi.stack_in).unwrap_or(i32::MAX);
        let out_produce = i32::try_from(opi.stack_out).unwrap_or(i32::MAX);
        if stack_depth < in_need {
            return false;
        }
        stack_depth = stack_depth - in_need + out_produce;

        match opv {
            op::JUMP
            | op::JUMPI
            | op::STOP
            | op::RETURN
            | op::REVERT
            | op::INVALID
            | op::SELFDESTRUCT => return true,
            _ => {}
        }

        let Some(next_pc) = pc.checked_add(opi.size) else {
            return false;
        };
        if next_pc <= pc {
            return false;
        }
        pc = next_pc;
    }

    true
}

#[derive(Clone, Copy)]
struct StaticTraceOp {
    op: u8,
    known: bool,
    stack_in: usize,
    stack_out: usize,
    push32: Option<EventSelector>,
}

fn static_trace_ops(code: &[u8]) -> Vec<StaticTraceOp> {
    let mut ops = Vec::new();
    for (pc, cop) in iterate_code(code, 0, None) {
        let push32 = if cop.op == op::PUSH32 && pc + 33 <= code.len() {
            let mut topic = [0u8; 32];
            topic.copy_from_slice(&code[pc + 1..pc + 33]);
            Some(topic)
        } else {
            None
        };
        ops.push(StaticTraceOp {
            op: cop.op,
            known: cop.opi.known,
            stack_in: cop.opi.stack_in,
            stack_out: cop.opi.stack_out,
            push32,
        });
    }
    ops
}

fn is_inline_guard_trap(ops: &[StaticTraceOp], idx: usize) -> bool {
    let opv = ops[idx].op;
    if !matches!(opv, op::INVALID | op::REVERT) {
        return false;
    }
    let Some(prev) = idx.checked_sub(1).and_then(|i| ops.get(i)) else {
        return false;
    };
    let Some(next) = ops.get(idx + 1) else {
        return false;
    };
    prev.op == op::JUMPI && next.op == op::JUMPDEST
}

fn trace_topic0_push32_from_log(
    ops: &[StaticTraceOp],
    log_idx: usize,
    scan_window: usize,
    break_on_jump: bool,
) -> Option<EventSelector> {
    if log_idx == 0 {
        return None;
    }

    // Before LOGx executes, topic0 is 3rd stack item from the top:
    // [mstart, msize, topic0, ...]
    let mut tracked_slot = 2usize;
    let mut scanned = 0usize;
    let mut idx = log_idx;

    while idx > 0 && scanned < scan_window {
        idx -= 1;
        scanned += 1;
        let instr = ops[idx];

        if !instr.known {
            return None;
        }

        if matches!(instr.op, op::STOP | op::RETURN | op::SELFDESTRUCT) {
            break;
        }
        if matches!(instr.op, op::INVALID | op::REVERT) {
            // `JUMPI; INVALID/REVERT; JUMPDEST` is often an inline guard trap.
            // Treat it as a branch marker, not as a hard barrier.
            if is_inline_guard_trap(ops, idx) {
                continue;
            }
            break;
        }
        if break_on_jump {
            if instr.op == op::JUMP {
                break;
            }
        }

        match instr.op {
            op::PUSH32 => {
                if tracked_slot == 0 {
                    if let Some(topic) = instr.push32
                        && is_plausible_event_hash(&topic)
                    {
                        return Some(topic);
                    }
                    return None;
                }
                tracked_slot -= 1;
            }
            op::PUSH0 | op::PUSH1..=op::PUSH31 => {
                if tracked_slot == 0 {
                    return None;
                }
                tracked_slot -= 1;
            }
            op::DUP1..=op::DUP16 => {
                let n = (instr.op - op::DUP1 + 1) as usize;
                if tracked_slot == 0 {
                    tracked_slot = n - 1;
                } else if tracked_slot <= n - 1 {
                    tracked_slot -= 1;
                }
            }
            op::SWAP1..=op::SWAP16 => {
                let n = (instr.op - op::SWAP1 + 1) as usize;
                if tracked_slot == 0 {
                    tracked_slot = n;
                } else if tracked_slot == n {
                    tracked_slot = 0;
                }
            }
            op::POP => tracked_slot = tracked_slot.saturating_add(1),
            _ => {
                if tracked_slot < instr.stack_out {
                    return None;
                }
                tracked_slot = tracked_slot - instr.stack_out + instr.stack_in;
            }
        }

        if tracked_slot > 1024 {
            return None;
        }
    }

    None
}

fn static_event_candidates_from_push32_forward(
    ops: &[StaticTraceOp],
    scan_window: usize,
    break_on_jump: bool,
) -> HashSet<EventSelector> {
    let mut candidates: HashSet<EventSelector> = HashSet::default();
    for i in 0..ops.len() {
        let Some(topic0) = ops[i].push32 else {
            continue;
        };
        if !is_plausible_event_hash(&topic0) {
            continue;
        }

        let mut near_log = false;
        for opv in ops.iter().skip(i + 1).take(scan_window).map(|v| v.op) {
            if (op::LOG1..=op::LOG4).contains(&opv) {
                near_log = true;
                break;
            }
            if matches!(
                opv,
                op::STOP | op::RETURN | op::REVERT | op::INVALID | op::SELFDESTRUCT
            ) || (break_on_jump && matches!(opv, op::JUMP | op::JUMPI))
            {
                break;
            }
        }

        if near_log {
            candidates.insert(topic0);
        }
    }

    candidates
}

fn static_event_candidate_set(
    code: &[u8],
    scan_window: u8,
    break_on_jump: bool,
) -> HashSet<EventSelector> {
    if code.is_empty() {
        return HashSet::default();
    }

    let ops = static_trace_ops(code);
    let mut candidates: HashSet<EventSelector> = HashSet::default();
    for (idx, instr) in ops.iter().enumerate() {
        if !(op::LOG1..=op::LOG4).contains(&instr.op) {
            continue;
        }
        if let Some(topic) =
            trace_topic0_push32_from_log(&ops, idx, scan_window as usize, break_on_jump)
        {
            candidates.insert(topic);
        }
    }
    candidates
}

fn static_event_candidates(code: &[u8]) -> usize {
    if code.is_empty() {
        return 0;
    }
    let ops = static_trace_ops(code);
    static_event_candidates_from_push32_forward(
        &ops,
        STATIC_EVENT_SCAN_WINDOW_METRIC as usize,
        true,
    )
    .len()
}

fn static_supplement_window() -> u8 {
    std::env::var("EVMOLE_STATIC_SUPPLEMENT_WINDOW")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(STATIC_EVENT_SCAN_WINDOW_SUPPLEMENT)
}

fn parse_selector_hex(s: &str) -> Option<Selector> {
    let t = s.trim();
    let h = t.strip_prefix("0x").unwrap_or(t);
    if h.len() != 8 {
        return None;
    }
    let raw = alloy_primitives::hex::decode(h).ok()?;
    if raw.len() != 4 {
        return None;
    }
    let mut out = [0u8; 4];
    out.copy_from_slice(&raw);
    Some(out)
}

struct LogPathIndex {
    pc_to_block: Vec<usize>,
    edges_by_context: HashMap<usize, HashMap<usize, HashSet<usize>>>,
    distance_by_context: HashMap<usize, HashMap<usize, u16>>,
}

impl LogPathIndex {
    fn block_for_pc(&self, pc: usize) -> Option<usize> {
        self.pc_to_block
            .get(pc)
            .copied()
            .and_then(|v| if v == usize::MAX { None } else { Some(v) })
    }

    fn distance_for_pc(&self, context: usize, pc: usize) -> Option<u16> {
        let block = self.block_for_pc(pc)?;
        self.distance_by_context.get(&context)?.get(&block).copied()
    }
}

fn block_has_log(code: &[u8], block: &Block) -> bool {
    iterate_code(code, block.start, Some(block.end))
        .any(|(_, cop)| (op::LOG1..=op::LOG4).contains(&cop.op))
}

fn find_block_start(blocks: &BTreeMap<usize, Block>, pc: usize) -> Option<usize> {
    let (start, block) = blocks.range(..=pc).next_back()?;
    if pc <= block.end { Some(*start) } else { None }
}

fn build_log_path_index(code: &[u8], contexts: &[usize]) -> Option<LogPathIndex> {
    if code.is_empty() || contexts.is_empty() {
        return None;
    }

    let cfg = control_flow_graph(code, basic_blocks(code));
    if cfg.blocks.is_empty() {
        return None;
    }

    let mut pc_to_block = vec![usize::MAX; code.len()];
    for (start, block) in &cfg.blocks {
        for (pc, _) in iterate_code(code, *start, Some(block.end)) {
            pc_to_block[pc] = *start;
        }
    }

    let mut succ: HashMap<usize, HashSet<usize>> = HashMap::default();
    let mut pred: HashMap<usize, HashSet<usize>> = HashMap::default();
    let mut log_blocks: HashSet<usize> = HashSet::default();

    let mut add_edge = |from: usize, to: usize| {
        if to >= INVALID_JUMP_START || !cfg.blocks.contains_key(&to) {
            return;
        }
        succ.entry(from).or_default().insert(to);
        pred.entry(to).or_default().insert(from);
    };

    for (start, block) in &cfg.blocks {
        if block_has_log(code, block) {
            log_blocks.insert(*start);
        }
        match &block.btype {
            BlockType::Terminate { .. } => {}
            BlockType::Jump { to } => add_edge(*start, *to),
            BlockType::Jumpi { true_to, false_to } => {
                add_edge(*start, *true_to);
                add_edge(*start, *false_to);
            }
            BlockType::DynamicJump { to } => {
                for dj in to {
                    if let Some(dst) = dj.to {
                        add_edge(*start, dst);
                    }
                }
            }
            BlockType::DynamicJumpi { true_to, false_to } => {
                add_edge(*start, *false_to);
                for dj in true_to {
                    if let Some(dst) = dj.to {
                        add_edge(*start, dst);
                    }
                }
            }
        }
    }

    if log_blocks.is_empty() {
        return None;
    }

    let mut to_log: HashSet<usize> = HashSet::default();
    let mut stack: Vec<usize> = log_blocks.iter().copied().collect();
    while let Some(node) = stack.pop() {
        if !to_log.insert(node) {
            continue;
        }
        if let Some(parents) = pred.get(&node) {
            stack.extend(parents.iter().copied());
        }
    }

    let mut edges_by_context: HashMap<usize, HashMap<usize, HashSet<usize>>> = HashMap::default();
    let mut distance_by_context: HashMap<usize, HashMap<usize, u16>> = HashMap::default();
    for &context in contexts {
        let Some(entry_block) = find_block_start(&cfg.blocks, context) else {
            continue;
        };

        let mut from_entry: HashSet<usize> = HashSet::default();
        let mut queue = vec![entry_block];
        while let Some(node) = queue.pop() {
            if !from_entry.insert(node) {
                continue;
            }
            if let Some(nexts) = succ.get(&node) {
                queue.extend(nexts.iter().copied());
            }
        }

        let mut allowed: HashSet<usize> = HashSet::default();
        for node in from_entry {
            if to_log.contains(&node) {
                allowed.insert(node);
            }
        }

        if !allowed.is_empty() {
            let mut allowed_edges: HashMap<usize, HashSet<usize>> = HashMap::default();
            let mut allowed_pred: HashMap<usize, HashSet<usize>> = HashMap::default();
            for &from in &allowed {
                if let Some(nexts) = succ.get(&from) {
                    for &to in nexts {
                        if allowed.contains(&to) {
                            allowed_edges.entry(from).or_default().insert(to);
                            allowed_pred.entry(to).or_default().insert(from);
                        }
                    }
                }
            }

            if !allowed_edges.is_empty() {
                edges_by_context.insert(context, allowed_edges);
            }

            let mut distance: HashMap<usize, u16> = HashMap::default();
            let mut queue: VecDeque<usize> = VecDeque::new();
            for &node in &allowed {
                if log_blocks.contains(&node) {
                    distance.insert(node, 0);
                    queue.push_back(node);
                }
            }

            while let Some(node) = queue.pop_front() {
                let cur_dist = distance.get(&node).copied().unwrap_or(0);
                let next_dist = cur_dist.saturating_add(1);
                if let Some(preds) = allowed_pred.get(&node) {
                    for &pred_node in preds {
                        let update = distance
                            .get(&pred_node)
                            .is_none_or(|&existing| next_dist < existing);
                        if update {
                            distance.insert(pred_node, next_dist);
                            queue.push_back(pred_node);
                        }
                    }
                }
            }

            if !distance.is_empty() {
                distance_by_context.insert(context, distance);
            }
        }
    }

    Some(LogPathIndex {
        pc_to_block,
        edges_by_context,
        distance_by_context,
    })
}

fn should_run_recall_rounds(
    selectors_total: usize,
    pending_selectors: usize,
    found_events: usize,
    static_candidates: usize,
) -> bool {
    if pending_selectors == 0 {
        return false;
    }
    if found_events == 0 {
        return true;
    }
    if selectors_total == 0 {
        return static_candidates == 0 || found_events < static_candidates;
    }
    if pending_selectors < 2 {
        return false;
    }

    let pending_ratio = pending_selectors as f32 / selectors_total as f32;
    pending_ratio >= 0.02 || pending_selectors >= 4
}

fn probe_branch_cached(
    vm: &Vm<Label, CallDataImpl>,
    start_pc: usize,
    step_limit: u16,
    gas_limit: u32,
    context: usize,
    cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
    stats: &mut EventExtractionStats,
) -> ProbeOutcome {
    let key = probe_cache_key(vm, start_pc, context);
    if let Some(outcome) = cache.get(&key) {
        stats.probe_cache_hits += 1;
        return *outcome;
    }

    stats.probe_cache_misses += 1;
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
    prefer_other_on_tie: bool,
    probe_steps: u16,
    probe_gas: u32,
    static_dead_cache: &mut HashMap<usize, bool>,
    probe_cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
    stats: &mut EventExtractionStats,
) -> JumpClassify {
    if other_pc == vm.pc {
        return JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        };
    }

    // Solidity require() revert branches are often statically obvious:
    // JUMPDEST -> PUSH* ... -> REVERT/INVALID.
    let other_static_dead = is_static_dead_end_cached(vm.code, other_pc, static_dead_cache);
    if other_static_dead {
        stats.static_dead_other_prunes += 1;
        return JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        };
    }

    let current_static_dead = is_static_dead_end_cached(vm.code, vm.pc, static_dead_cache);
    if current_static_dead {
        stats.static_dead_current_prunes += 1;
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

    let other = probe_branch_cached(
        vm,
        other_pc,
        probe_steps,
        probe_gas,
        context,
        probe_cache,
        stats,
    );
    if other == ProbeOutcome::DeadEnd {
        return JumpClassify {
            decision: JumpDecision::KeepCurrent,
            needs_more: false,
        };
    }

    let current = probe_branch_cached(
        vm,
        vm.pc,
        probe_steps,
        probe_gas,
        context,
        probe_cache,
        stats,
    );
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
            decision: if prefer_other_on_tie {
                JumpDecision::SwitchOther
            } else {
                JumpDecision::KeepCurrent
            },
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

fn prefer_other_branch_by_log_distance(
    path_index: Option<&LogPathIndex>,
    context: usize,
    current_pc: usize,
    other_pc: usize,
) -> bool {
    let Some(index) = path_index else {
        return false;
    };
    let current_dist = index.distance_for_pc(context, current_pc);
    let other_dist = index.distance_for_pc(context, other_pc);
    match (current_dist, other_dist) {
        (Some(cd), Some(od)) => od < cd,
        (None, Some(_)) => true,
        _ => false,
    }
}

fn dynamic_jump_extra_targets(
    path_index: Option<&LogPathIndex>,
    context: usize,
    step_pc: usize,
    current_pc: usize,
) -> Vec<usize> {
    let Some(index) = path_index else {
        return Vec::new();
    };
    let Some(src_block) = index.block_for_pc(step_pc) else {
        return Vec::new();
    };
    let Some(current_block) = index.block_for_pc(current_pc) else {
        return Vec::new();
    };
    let Some(edges) = index.edges_by_context.get(&context) else {
        return Vec::new();
    };
    let Some(nexts) = edges.get(&src_block) else {
        return Vec::new();
    };
    if nexts.len() < DYNAMIC_JUMP_HOT_FANOUT_MIN {
        return Vec::new();
    }

    let mut log_targets: Vec<(u16, usize)> = Vec::new();
    let mut other_targets: Vec<(u16, usize)> = Vec::new();

    for &target in nexts {
        if target == current_block || target == src_block {
            continue;
        }
        let Some(dist) = index.distance_for_pc(context, target) else {
            continue;
        };
        if dist == 0 {
            log_targets.push((dist, target));
        } else {
            other_targets.push((dist, target));
        }
    }

    log_targets.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    other_targets.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut out = Vec::with_capacity(log_targets.len() + other_targets.len());
    if let Some((_, target)) = log_targets.first() {
        out.push(*target);
    }
    if let Some((_, target)) = other_targets.first() {
        out.push(*target);
    }
    out.extend(log_targets.into_iter().skip(1).map(|(_, target)| target));
    out.extend(other_targets.into_iter().skip(1).map(|(_, target)| target));
    out
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
    may_reach_log: &[bool],
    path_index: Option<&LogPathIndex>,
    static_dead_cache: &mut HashMap<usize, bool>,
    probe_cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
    jump_classify_cache: &mut HashMap<JumpClassifyCacheKey, JumpClassify>,
    stats: &mut EventExtractionStats,
    profile: &mut Option<&mut EventExecutionProfile>,
    gas_limit: u32,
    max_depth: u8,
    max_steps: u32,
    max_visited_states: usize,
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
        may_reach_log,
        path_index,
        static_dead_cache,
        probe_cache,
        jump_classify_cache,
        stats,
        profile,
        gas_limit,
        max_depth,
        max_steps,
        max_visited_states,
        None,
    );
    needs_more.into_iter().next().unwrap_or(false)
}

fn execute_paths_batch<'a>(
    initial_states: Vec<BatchState<'a>>,
    states_count: usize,
    events: &mut Vec<EventSelector>,
    seen: &mut HashSet<EventSelector>,
    may_reach_log: &[bool],
    path_index: Option<&LogPathIndex>,
    static_dead_cache: &mut HashMap<usize, bool>,
    probe_cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
    jump_classify_cache: &mut HashMap<JumpClassifyCacheKey, JumpClassify>,
    stats: &mut EventExtractionStats,
    profile: &mut Option<&mut EventExecutionProfile>,
    gas_limit: u32,
    max_depth: u8,
    max_steps: u32,
    max_visited_states: usize,
    mut visited_cap_hit_by_idx: Option<&mut [bool]>,
) -> Vec<bool> {
    let disable_reachability_prune =
        std::env::var_os("EVMOLE_DISABLE_REACHABILITY_PRUNE").is_some();
    let disable_context_edge_prune =
        std::env::var_os("EVMOLE_DISABLE_CONTEXT_EDGE_PRUNE").is_some();
    let enable_dynamic_jump_second_pass =
        std::env::var_os("EVMOLE_DISABLE_DYNAMIC_JUMP_SECOND_PASS").is_none();
    let disable_fork_throttle = std::env::var_os("EVMOLE_DISABLE_FORK_THROTTLE").is_some();
    let disable_fork_dedupe = std::env::var_os("EVMOLE_DISABLE_FORK_DEDUPE").is_some();
    let enable_call_fail_fork = std::env::var_os("EVMOLE_DISABLE_CALL_FAIL_FORK").is_none();
    let trace_log_pc = std::env::var("EVMOLE_TRACE_LOG_PC").ok().and_then(|v| {
        let s = v.trim();
        let h = s.strip_prefix("0x").unwrap_or(s);
        usize::from_str_radix(h, 16).ok()
    });
    let trace_pc = std::env::var("EVMOLE_TRACE_PC").ok().and_then(|v| {
        let s = v.trim();
        let h = s.strip_prefix("0x").unwrap_or(s);
        usize::from_str_radix(h, 16).ok()
    });
    let trace_call_cond = std::env::var_os("EVMOLE_TRACE_CALL_COND").is_some();

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
    if let Some(p) = profile.as_deref_mut() {
        p.states_pushed = p.states_pushed.saturating_add(queue.len() as u64);
        p.queue_peak = p.queue_peak.max(queue.len());
    }

    if queue.is_empty() {
        return needs_more;
    }

    let mut visited: HashSet<StateKey> = HashSet::default();
    let mut jumpi_fork_counts: HashMap<(usize, usize), u16> = HashMap::default();
    let mut jumpi_fork_dedupe_counts: HashMap<ForkDedupeKey, u8> = HashMap::default();
    let mut dynamic_jump_fork_counts: HashMap<(usize, usize), u16> = HashMap::default();
    let mut dynamic_jump_target_counts: HashMap<(usize, usize, usize), u8> = HashMap::default();
    let mut call_fail_fork_counts: HashMap<(usize, usize), u8> = HashMap::default();
    while let Some(state) = queue.pop() {
        if let Some(p) = profile.as_deref_mut() {
            p.states_popped = p.states_popped.saturating_add(1);
        }
        let idx = state.idx;
        let context = state.context;
        let mut vm = state.vm;
        let mut gas_used = state.gas_used;
        let depth = state.depth;
        let mut steps = state.steps;

        while !vm.stopped {
            if gas_used >= gas_limit || steps >= max_steps {
                needs_more[idx] = true;
                if let Some(p) = profile.as_deref_mut() {
                    p.state_limit_breaks = p.state_limit_breaks.saturating_add(1);
                }
                break;
            }

            let step_pc = vm.pc;
            let ret = match vm.step() {
                Ok(v) => v,
                Err(_) => break,
            };

            if trace_pc == Some(step_pc) {
                eprintln!(
                    "[trace-pc] pc=0x{step_pc:x} op={} next_pc=0x{:x} context=0x{:x} depth={} gas={} steps={} stack_len={}",
                    op::info(ret.op).name,
                    vm.pc,
                    context,
                    depth,
                    gas_used,
                    steps,
                    vm.stack.data.len()
                );
            }

            gas_used = gas_used.saturating_add(ret.gas_used);
            steps += 1;

            if gas_used > gas_limit {
                break;
            }

            match ret.op {
                op::LOG1..=op::LOG4 => {
                    if trace_log_pc == Some(step_pc) {
                        eprintln!(
                            "[trace-log-pc] pc=0x{step_pc:x} op=0x{:x} topic0={:02x?}",
                            ret.op, ret.args[0].data
                        );
                    }
                    collect_event(events, seen, ret.args[0].data)
                }
                op::CALL | op::CALLCODE | op::DELEGATECALL | op::STATICCALL => {
                    if enable_call_fail_fork && let Ok(top) = vm.stack.peek_mut() {
                        top.label = Some(Label::ExternalCallResult(step_pc));
                    }
                }
                op::JUMPI => {
                    let trace_this_jump = trace_pc == Some(step_pc);
                    if let Some(p) = profile.as_deref_mut() {
                        p.jumpi_total = p.jumpi_total.saturating_add(1);
                        bump_pc(&mut p.jumpi_by_pc, step_pc);
                    }
                    if visited.len() >= max_visited_states {
                        needs_more[idx] = true;
                        if let Some(by_idx) = visited_cap_hit_by_idx.as_deref_mut()
                            && let Some(hit) = by_idx.get_mut(idx)
                        {
                            *hit = true;
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.visited_cap_hits = p.visited_cap_hits.saturating_add(1);
                        }
                        break;
                    }
                    let jump_state = state_key(&vm, context);
                    if !visited.insert(jump_state) {
                        if trace_this_jump {
                            eprintln!(
                                "[trace-jumpi] pc=0x{step_pc:x} visited_break current_pc=0x{:x}",
                                vm.pc
                            );
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.jumpi_visited_breaks = p.jumpi_visited_breaks.saturating_add(1);
                            bump_pc(&mut p.jumpi_visited_breaks_by_pc, step_pc);
                        }
                        break;
                    }

                    let cond_zero = ret.args[1].data == [0u8; 32];
                    let other_pc = if cond_zero {
                        usize::try_from(&ret.args[0]).ok()
                    } else {
                        step_pc.checked_add(1)
                    };

                    let Some(other_pc) = other_pc else {
                        if trace_this_jump {
                            eprintln!(
                                "[trace-jumpi] pc=0x{step_pc:x} invalid_other_pc cond_zero={} current_pc=0x{:x}",
                                cond_zero, vm.pc
                            );
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.jumpi_invalid_other_pc = p.jumpi_invalid_other_pc.saturating_add(1);
                            bump_pc(&mut p.jumpi_invalid_other_pc_by_pc, step_pc);
                        }
                        continue;
                    };

                    let other_is_valid = if cond_zero {
                        other_pc < vm.code.len() && vm.code[other_pc] == op::JUMPDEST
                    } else {
                        other_pc < vm.code.len()
                    };

                    if !other_is_valid {
                        if trace_this_jump {
                            eprintln!(
                                "[trace-jumpi] pc=0x{step_pc:x} other_not_valid cond_zero={} other_pc=0x{other_pc:x} current_pc=0x{:x}",
                                cond_zero, vm.pc
                            );
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.jumpi_invalid_other_pc = p.jumpi_invalid_other_pc.saturating_add(1);
                            bump_pc(&mut p.jumpi_invalid_other_pc_by_pc, step_pc);
                        }
                        continue;
                    }

                    let call_cond_key = if enable_call_fail_fork {
                        if let Some(Label::ExternalCallResult(call_pc)) = ret.args[1].label.as_ref()
                        {
                            Some((context, *call_pc))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if trace_call_cond && let Some((_, call_pc)) = call_cond_key {
                        eprintln!(
                            "[trace-call-cond] jump_pc=0x{step_pc:x} call_pc=0x{call_pc:x} cond={:02x?}",
                            ret.args[1].data
                        );
                    }

                    let mut current_can_reach = if disable_reachability_prune {
                        true
                    } else {
                        may_reach_log.get(vm.pc).copied().unwrap_or(false)
                    };
                    let mut other_can_reach = if disable_reachability_prune {
                        true
                    } else {
                        may_reach_log.get(other_pc).copied().unwrap_or(false)
                    };
                    if call_cond_key.is_some() {
                        current_can_reach = true;
                        other_can_reach = true;
                    }
                    if !disable_context_edge_prune && let Some(index) = path_index {
                        if let (Some(src_block), Some(current_block), Some(other_block)) = (
                            index.block_for_pc(step_pc),
                            index.block_for_pc(vm.pc),
                            index.block_for_pc(other_pc),
                        ) {
                            if let Some(edges) = index.edges_by_context.get(&context)
                                && let Some(nexts) = edges.get(&src_block)
                            {
                                let current_edge_ok = nexts.contains(&current_block);
                                let other_edge_ok = nexts.contains(&other_block);
                                if current_edge_ok && !other_edge_ok {
                                    other_can_reach = false;
                                } else if !current_edge_ok && other_edge_ok {
                                    current_can_reach = false;
                                }
                            }
                        }
                    }
                    if !current_can_reach && !other_can_reach {
                        if trace_this_jump {
                            eprintln!(
                                "[trace-jumpi] pc=0x{step_pc:x} prune=both current_pc=0x{:x} other_pc=0x{other_pc:x}",
                                vm.pc
                            );
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.jumpi_unreachable_both = p.jumpi_unreachable_both.saturating_add(1);
                            bump_pc(&mut p.jumpi_unreachable_both_by_pc, step_pc);
                        }
                        break;
                    }
                    if !current_can_reach {
                        if trace_this_jump {
                            eprintln!(
                                "[trace-jumpi] pc=0x{step_pc:x} prune=current current_pc=0x{:x} other_pc=0x{other_pc:x}",
                                vm.pc
                            );
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.jumpi_unreachable_current =
                                p.jumpi_unreachable_current.saturating_add(1);
                            bump_pc(&mut p.jumpi_unreachable_current_by_pc, step_pc);
                        }
                        vm.pc = other_pc;
                        continue;
                    }
                    if !other_can_reach {
                        if trace_this_jump {
                            eprintln!(
                                "[trace-jumpi] pc=0x{step_pc:x} prune=other current_pc=0x{:x} other_pc=0x{other_pc:x}",
                                vm.pc
                            );
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.jumpi_unreachable_other = p.jumpi_unreachable_other.saturating_add(1);
                            bump_pc(&mut p.jumpi_unreachable_other_by_pc, step_pc);
                        }
                        continue;
                    }

                    let probe_gas = gas_limit.saturating_sub(gas_used).min(PROBE_GAS_LIMIT);
                    let can_fork_raw = depth < max_depth && queue.len() < MAX_PENDING_STATES;
                    let mut can_fork = can_fork_raw;
                    let prefer_other_on_tie =
                        prefer_other_branch_by_log_distance(path_index, context, vm.pc, other_pc);
                    let mut fork_dedupe_key_for_enqueue = None;
                    let mut call_fail_fork_key_for_enqueue = None;
                    let force_call_cond_fork = call_cond_key.is_some_and(|key| {
                        call_fail_fork_counts.get(&key).copied().unwrap_or(0)
                            < MAX_CALL_FAIL_FORKS_PER_CONTEXT_PC
                    });
                    if can_fork_raw && !force_call_cond_fork && !disable_fork_throttle {
                        let fork_used = jumpi_fork_counts
                            .get(&(context, step_pc))
                            .copied()
                            .unwrap_or(0);
                        if fork_used >= MAX_JUMPI_FORKS_PER_CONTEXT_PC {
                            can_fork = false;
                            if let Some(p) = profile.as_deref_mut() {
                                p.jumpi_fork_throttled = p.jumpi_fork_throttled.saturating_add(1);
                                bump_pc(&mut p.jumpi_fork_throttled_by_pc, step_pc);
                            }
                        }
                    }
                    if can_fork && !force_call_cond_fork && !disable_fork_dedupe {
                        let dedupe_key = fork_dedupe_key(&vm, context, step_pc, other_pc);
                        let used = jumpi_fork_dedupe_counts
                            .get(&dedupe_key)
                            .copied()
                            .unwrap_or(0);
                        if used >= MAX_JUMPI_FORKS_PER_EQUIV_KEY {
                            can_fork = false;
                            if let Some(p) = profile.as_deref_mut() {
                                p.jumpi_fork_deduped = p.jumpi_fork_deduped.saturating_add(1);
                                bump_pc(&mut p.jumpi_fork_deduped_by_pc, step_pc);
                            }
                        } else {
                            fork_dedupe_key_for_enqueue = Some(dedupe_key);
                        }
                    }
                    if force_call_cond_fork && can_fork {
                        call_fail_fork_key_for_enqueue = call_cond_key;
                    }

                    let jump = if force_call_cond_fork {
                        if can_fork {
                            JumpClassify {
                                decision: JumpDecision::ForkBoth,
                                needs_more: false,
                            }
                        } else {
                            JumpClassify {
                                decision: JumpDecision::KeepCurrent,
                                needs_more: true,
                            }
                        }
                    } else if can_fork {
                        if let Some(p) = profile.as_deref_mut() {
                            bump_pc(&mut p.jumpi_can_fork_true_by_pc, step_pc);
                        }
                        stats.jump_classify_can_fork_true += 1;
                        classify_jump(
                            &vm,
                            context,
                            other_pc,
                            true,
                            prefer_other_on_tie,
                            PROBE_STEP_LIMIT,
                            probe_gas,
                            static_dead_cache,
                            probe_cache,
                            stats,
                        )
                    } else {
                        if let Some(p) = profile.as_deref_mut() {
                            bump_pc(&mut p.jumpi_can_fork_false_by_pc, step_pc);
                        }
                        stats.jump_classify_can_fork_false += 1;
                        let cache_key = JumpClassifyCacheKey {
                            state: jump_state,
                            step_pc,
                            other_pc,
                            prefer_other_on_tie,
                            probe_steps: PROBE_STEP_LIMIT,
                            probe_gas,
                        };
                        if let Some(cached) = jump_classify_cache.get(&cache_key) {
                            if let Some(p) = profile.as_deref_mut() {
                                bump_pc(&mut p.jumpi_cache_hit_by_pc, step_pc);
                            }
                            stats.jump_classify_cache_hits += 1;
                            *cached
                        } else {
                            if let Some(p) = profile.as_deref_mut() {
                                bump_pc(&mut p.jumpi_cache_miss_by_pc, step_pc);
                            }
                            stats.jump_classify_cache_misses += 1;
                            let computed = classify_jump(
                                &vm,
                                context,
                                other_pc,
                                false,
                                prefer_other_on_tie,
                                PROBE_STEP_LIMIT,
                                probe_gas,
                                static_dead_cache,
                                probe_cache,
                                stats,
                            );
                            if jump_classify_cache.len() >= MAX_JUMP_CLASSIFY_CACHE {
                                jump_classify_cache.clear();
                            }
                            jump_classify_cache.insert(cache_key, computed);
                            computed
                        }
                    };
                    if jump.needs_more {
                        needs_more[idx] = true;
                    }

                    match jump.decision {
                        JumpDecision::KeepCurrent => {
                            if trace_this_jump {
                                eprintln!(
                                    "[trace-jumpi] pc=0x{step_pc:x} decision=keep current_pc=0x{:x} other_pc=0x{other_pc:x}",
                                    vm.pc
                                );
                            }
                            if let Some(p) = profile.as_deref_mut() {
                                p.jumpi_decision_keep = p.jumpi_decision_keep.saturating_add(1);
                                bump_pc(&mut p.jumpi_decision_keep_by_pc, step_pc);
                            }
                        }
                        JumpDecision::SwitchOther => {
                            if trace_this_jump {
                                eprintln!(
                                    "[trace-jumpi] pc=0x{step_pc:x} decision=switch current_pc=0x{:x} other_pc=0x{other_pc:x}",
                                    vm.pc
                                );
                            }
                            if let Some(p) = profile.as_deref_mut() {
                                p.jumpi_decision_switch = p.jumpi_decision_switch.saturating_add(1);
                                bump_pc(&mut p.jumpi_decision_switch_by_pc, step_pc);
                            }
                            vm.pc = other_pc
                        }
                        JumpDecision::ForkBoth => {
                            if trace_this_jump {
                                eprintln!(
                                    "[trace-jumpi] pc=0x{step_pc:x} decision=fork current_pc=0x{:x} other_pc=0x{other_pc:x} can_fork={}",
                                    vm.pc, can_fork
                                );
                            }
                            if let Some(p) = profile.as_deref_mut() {
                                p.jumpi_decision_fork = p.jumpi_decision_fork.saturating_add(1);
                                bump_pc(&mut p.jumpi_decision_fork_by_pc, step_pc);
                            }
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
                                if let Some(p) = profile.as_deref_mut() {
                                    p.states_pushed = p.states_pushed.saturating_add(1);
                                    p.queue_peak = p.queue_peak.max(queue.len());
                                }
                                let key = (context, step_pc);
                                let used = jumpi_fork_counts.entry(key).or_insert(0);
                                *used = used.saturating_add(1);
                                if let Some(dedupe_key) = fork_dedupe_key_for_enqueue {
                                    let used =
                                        jumpi_fork_dedupe_counts.entry(dedupe_key).or_insert(0);
                                    *used = used.saturating_add(1);
                                }
                                if let Some(key) = call_fail_fork_key_for_enqueue {
                                    let used = call_fail_fork_counts.entry(key).or_insert(0);
                                    *used = used.saturating_add(1);
                                }
                            }
                        }
                    }
                }
                op::JUMP => {
                    if let Some(p) = profile.as_deref_mut() {
                        p.jump_total = p.jump_total.saturating_add(1);
                    }
                    if visited.len() >= max_visited_states {
                        needs_more[idx] = true;
                        if let Some(by_idx) = visited_cap_hit_by_idx.as_deref_mut()
                            && let Some(hit) = by_idx.get_mut(idx)
                        {
                            *hit = true;
                        }
                        if let Some(p) = profile.as_deref_mut() {
                            p.visited_cap_hits = p.visited_cap_hits.saturating_add(1);
                        }
                        break;
                    }
                    if !visited.insert(state_key(&vm, context)) {
                        if let Some(p) = profile.as_deref_mut() {
                            p.jump_visited_breaks = p.jump_visited_breaks.saturating_add(1);
                        }
                        break;
                    }

                    if enable_dynamic_jump_second_pass {
                        let extra_targets =
                            dynamic_jump_extra_targets(path_index, context, step_pc, vm.pc);
                        if !extra_targets.is_empty() {
                            let can_fork = depth < max_depth && queue.len() < MAX_PENDING_STATES;
                            if !can_fork {
                                needs_more[idx] = true;
                                continue;
                            }

                            let site_key = (context, step_pc);
                            let site_used = dynamic_jump_fork_counts
                                .get(&site_key)
                                .copied()
                                .unwrap_or(0);
                            if site_used >= MAX_DYNAMIC_JUMP_EXTRA_FORKS_PER_SITE {
                                needs_more[idx] = true;
                                continue;
                            }

                            let mut pushed = 0u16;
                            for target_pc in extra_targets {
                                if pushed >= DYNAMIC_JUMP_EXTRA_TARGETS_PER_HIT as u16
                                    || queue.len() >= MAX_PENDING_STATES
                                    || depth >= max_depth
                                {
                                    break;
                                }
                                let target_key = (context, step_pc, target_pc);
                                let target_used = dynamic_jump_target_counts
                                    .get(&target_key)
                                    .copied()
                                    .unwrap_or(0);
                                if target_used >= MAX_DYNAMIC_JUMP_EXTRA_FORKS_PER_TARGET {
                                    continue;
                                }

                                let mut forked = vm.fork();
                                forked.pc = target_pc;
                                queue.push(BatchState {
                                    idx,
                                    context,
                                    vm: forked,
                                    gas_used,
                                    depth: depth + 1,
                                    steps,
                                });
                                if let Some(p) = profile.as_deref_mut() {
                                    p.states_pushed = p.states_pushed.saturating_add(1);
                                    p.queue_peak = p.queue_peak.max(queue.len());
                                }

                                let used =
                                    dynamic_jump_target_counts.entry(target_key).or_insert(0);
                                *used = used.saturating_add(1);
                                pushed = pushed.saturating_add(1);
                            }

                            if pushed > 0 {
                                let used = dynamic_jump_fork_counts.entry(site_key).or_insert(0);
                                *used = used.saturating_add(pushed);
                            }
                        }
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
    may_reach_log: &[bool],
    path_index: Option<&LogPathIndex>,
    static_dead_cache: &mut HashMap<usize, bool>,
    probe_cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
    jump_classify_cache: &mut HashMap<JumpClassifyCacheKey, JumpClassify>,
    stats: &mut EventExtractionStats,
    profile: &mut Option<&mut EventExecutionProfile>,
    gas_limit: u32,
    max_depth: u8,
    max_steps: u32,
    max_visited_states: usize,
) -> bool {
    let vm = Vm::new(code, calldata);
    execute_paths(
        vm,
        0,
        events,
        seen,
        may_reach_log,
        path_index,
        static_dead_cache,
        probe_cache,
        jump_classify_cache,
        stats,
        profile,
        gas_limit,
        max_depth,
        max_steps,
        max_visited_states,
    )
}

fn contract_events_with_stats_internal(
    code: &[u8],
    mut profile: Option<&mut EventExecutionProfile>,
) -> (Vec<EventSelector>, EventExtractionStats) {
    let mut stats = EventExtractionStats::default();
    if code.is_empty() {
        return (Vec::new(), stats);
    }
    if std::env::var_os("EVMOLE_EVENTS_BOTTOM_UP").is_some()
        && std::env::var_os("EVMOLE_DISABLE_EVENTS_BOTTOM_UP").is_none()
    {
        return (bottom_up::contract_events_bottom_up(code), stats);
    }

    struct BatchRoundResult {
        pending: Vec<([u8; 4], usize)>,
        starved_pending: Vec<([u8; 4], usize)>,
        had_visited_cap_hits: bool,
    }

    fn run_batch_round(
        code: &[u8],
        pending: Vec<([u8; 4], usize)>,
        allow_direct_entry: bool,
        events: &mut Vec<EventSelector>,
        seen: &mut HashSet<EventSelector>,
        may_reach_log: &[bool],
        path_index: Option<&LogPathIndex>,
        static_dead_cache: &mut HashMap<usize, bool>,
        probe_cache: &mut HashMap<ProbeCacheKey, ProbeOutcome>,
        entry_state_cache: &mut HashMap<EntryStateCacheKey, EntryStateSnapshot>,
        jump_classify_cache: &mut HashMap<JumpClassifyCacheKey, JumpClassify>,
        stats: &mut EventExtractionStats,
        profile: &mut Option<&mut EventExecutionProfile>,
        gas_limit: u32,
        max_depth: u8,
        max_steps: u32,
        max_visited_states: usize,
    ) -> BatchRoundResult {
        if pending.is_empty() {
            return BatchRoundResult {
                pending,
                starved_pending: Vec::new(),
                had_visited_cap_hits: false,
            };
        }

        let calldatas: Vec<CallDataImpl> = pending
            .iter()
            .map(|(selector, _)| CallDataImpl {
                selector: *selector,
            })
            .collect();

        let mut initial_states = Vec::with_capacity(pending.len());
        let mut round_needs_more = vec![false; pending.len()];
        let mut round_visited_cap_hits = vec![false; pending.len()];

        for (idx, ((selector, offset), calldata)) in
            pending.iter().zip(calldatas.iter()).enumerate()
        {
            if allow_direct_entry && is_safe_direct_function_entry(code, *offset) {
                let mut vm = Vm::new(code, calldata);
                vm.pc = *offset;
                if let Some(p) = profile.as_deref_mut() {
                    bump_pc(&mut p.context_start_by_pc, *offset);
                }
                initial_states.push(BatchState {
                    idx,
                    context: *offset,
                    vm,
                    gas_used: 0,
                    depth: 0,
                    steps: 0,
                });
                continue;
            }

            let entry_key = EntryStateCacheKey {
                selector: *selector,
                offset: *offset,
            };
            if let Some(snapshot) = entry_state_cache.get(&entry_key)
                && snapshot.gas_used <= gas_limit
            {
                stats.entry_state_cache_hits += 1;
                if let Some(p) = profile.as_deref_mut() {
                    bump_pc(&mut p.context_start_by_pc, *offset);
                }
                initial_states.push(BatchState {
                    idx,
                    context: *offset,
                    vm: restore_entry_state(code, calldata, snapshot),
                    gas_used: snapshot.gas_used,
                    depth: 0,
                    steps: 0,
                });
                continue;
            }
            stats.entry_state_cache_misses += 1;

            let mut vm = Vm::new(code, calldata);
            if let Some(initial_gas) = execute_until_function_start(&mut vm, gas_limit) {
                if entry_state_cache.len() >= MAX_ENTRY_STATE_CACHE {
                    entry_state_cache.clear();
                }
                entry_state_cache.insert(entry_key, snapshot_entry_state(&vm, initial_gas));
                if let Some(p) = profile.as_deref_mut() {
                    bump_pc(&mut p.context_start_by_pc, *offset);
                }
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

        let batch_needs_more = execute_paths_batch(
            initial_states,
            pending.len(),
            events,
            seen,
            may_reach_log,
            path_index,
            static_dead_cache,
            probe_cache,
            jump_classify_cache,
            stats,
            profile,
            gas_limit,
            max_depth,
            max_steps,
            max_visited_states,
            Some(&mut round_visited_cap_hits),
        );

        for (dst, src) in round_needs_more.iter_mut().zip(batch_needs_more.iter()) {
            *dst |= *src;
        }

        let had_visited_cap_hits = round_visited_cap_hits.iter().any(|v| *v);
        let mut pending_out = Vec::new();
        let mut starved_pending = Vec::new();
        for (idx, item) in pending.into_iter().enumerate() {
            if !round_needs_more[idx] {
                continue;
            }
            if round_visited_cap_hits[idx] {
                starved_pending.push(item);
            }
            pending_out.push(item);
        }

        BatchRoundResult {
            pending: pending_out,
            starved_pending,
            had_visited_cap_hits,
        }
    }

    let may_reach_log = compute_may_reach_log(code);
    let static_candidates = static_event_candidates(code);
    let (selectors_all, _) = crate::selectors::function_selectors(code, 0);
    let selectors_all_vec: Vec<(Selector, usize)> = selectors_all.into_iter().collect();
    let allow_direct_entry = std::env::var_os("EVMOLE_ENABLE_DIRECT_ENTRY").is_some()
        && std::env::var_os("EVMOLE_DISABLE_DIRECT_ENTRY").is_none();
    let base_max_visited_states = std::env::var("EVMOLE_MAX_VISITED_STATES")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(MAX_VISITED_STATES);
    let starvation_uplift_max_visited_states =
        std::env::var("EVMOLE_STARVATION_UPLIFT_MAX_VISITED_STATES")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or_else(|| {
                STARVATION_LOCAL_UPLIFT_MAX_VISITED_STATES.max(base_max_visited_states * 2)
            });
    let disable_starvation_local_uplift =
        std::env::var_os("EVMOLE_DISABLE_STARVATION_LOCAL_UPLIFT").is_some();
    let trace_starvation_local_uplift =
        std::env::var_os("EVMOLE_TRACE_STARVATION_LOCAL_UPLIFT").is_some();
    stats.selectors_total = selectors_all_vec.len() as u64;
    let has_any_selector = !selectors_all_vec.is_empty();
    let mut selectors = selectors_all_vec;
    let only_selector = std::env::var("EVMOLE_ONLY_SELECTOR")
        .ok()
        .and_then(|v| parse_selector_hex(&v));
    if let Some(sel) = only_selector {
        selectors.retain(|(s, _)| *s == sel);
    }
    if std::env::var_os("EVMOLE_FORCE_FALLBACK_SCAN").is_some() {
        selectors.clear();
    }
    let extra_fallback_round = std::env::var_os("EVMOLE_EXTRA_FALLBACK_ROUND").is_some();
    stats.selectors_after_mutability_prune = selectors.len() as u64;
    stats.selectors_pruned_view_or_pure = 0;
    let contexts: Vec<usize> = selectors.iter().map(|(_, offset)| *offset).collect();
    let path_index = build_log_path_index(code, &contexts);
    let mut events = Vec::<EventSelector>::new();
    let mut seen = HashSet::<EventSelector>::default();
    let mut static_dead_cache: HashMap<usize, bool> = HashMap::default();
    let mut probe_cache: HashMap<ProbeCacheKey, ProbeOutcome> = HashMap::default();
    let mut entry_state_cache: HashMap<EntryStateCacheKey, EntryStateSnapshot> = HashMap::default();
    let mut jump_classify_cache: HashMap<JumpClassifyCacheKey, JumpClassify> = HashMap::default();
    let mut stable_rounds = 0u8;

    if selectors.is_empty() {
        let calldata = CallDataImpl { selector: [0; 4] };
        let mut pending = true;
        for &(gas_limit, max_depth, max_steps) in &FAST_EXEC_ROUNDS {
            if !pending {
                break;
            }
            let before = seen.len();
            pending = execute_from_entry(
                code,
                &calldata,
                &mut events,
                &mut seen,
                &may_reach_log,
                path_index.as_ref(),
                &mut static_dead_cache,
                &mut probe_cache,
                &mut jump_classify_cache,
                &mut stats,
                &mut profile,
                gas_limit,
                max_depth,
                max_steps,
                base_max_visited_states,
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

        if pending && should_run_recall_rounds(0, 1, seen.len(), static_candidates) {
            let may_reach_all = vec![true; code.len()];
            let recall_rounds: &[(u32, u8, u32)] = if seen.is_empty() {
                &RECALL_EXEC_ROUNDS
            } else {
                &RECALL_EXEC_ROUNDS_PARTIAL
            };
            for &(gas_limit, max_depth, max_steps) in recall_rounds {
                if !pending {
                    break;
                }
                pending = execute_from_entry(
                    code,
                    &calldata,
                    &mut events,
                    &mut seen,
                    &may_reach_all,
                    path_index.as_ref(),
                    &mut static_dead_cache,
                    &mut probe_cache,
                    &mut jump_classify_cache,
                    &mut stats,
                    &mut profile,
                    gas_limit,
                    max_depth,
                    max_steps,
                    base_max_visited_states,
                );
            }
        }
    } else {
        let mut pending: Vec<([u8; 4], usize)> = selectors
            .iter()
            .map(|(selector, offset)| (*selector, *offset))
            .collect();
        let mut starvation_pending: Vec<([u8; 4], usize)> = Vec::new();
        let mut had_visited_cap_hits = false;

        for &(gas_limit, max_depth, max_steps) in &FAST_EXEC_ROUNDS {
            if pending.is_empty() {
                break;
            }
            let before = seen.len();
            let round = run_batch_round(
                code,
                pending,
                allow_direct_entry,
                &mut events,
                &mut seen,
                &may_reach_log,
                path_index.as_ref(),
                &mut static_dead_cache,
                &mut probe_cache,
                &mut entry_state_cache,
                &mut jump_classify_cache,
                &mut stats,
                &mut profile,
                gas_limit,
                max_depth,
                max_steps,
                base_max_visited_states,
            );
            pending = round.pending;
            starvation_pending = round.starved_pending;
            had_visited_cap_hits |= round.had_visited_cap_hits;

            if seen.len() == before {
                stable_rounds += 1;
                if stable_rounds >= 2 {
                    break;
                }
            } else {
                stable_rounds = 0;
            }
        }

        if !pending.is_empty()
            && !disable_starvation_local_uplift
            && had_visited_cap_hits
            && should_run_starvation_local_uplift(
                selectors.len(),
                pending.len(),
                starvation_pending.len(),
            )
        {
            if trace_starvation_local_uplift {
                eprintln!(
                    "[trace-starvation-uplift] trigger pending={} starved={} max_visited={} uplift_max={}",
                    pending.len(),
                    starvation_pending.len(),
                    base_max_visited_states,
                    starvation_uplift_max_visited_states
                );
            }
            let mut uplift_candidates = starvation_pending.clone();
            uplift_candidates.sort_by(|a, b| {
                let da = path_index
                    .as_ref()
                    .and_then(|index| index.distance_for_pc(a.1, a.1))
                    .unwrap_or(u16::MAX);
                let db = path_index
                    .as_ref()
                    .and_then(|index| index.distance_for_pc(b.1, b.1))
                    .unwrap_or(u16::MAX);
                da.cmp(&db).then_with(|| a.1.cmp(&b.1))
            });

            let (gas_limit, max_depth, max_steps) = FAST_EXEC_ROUNDS[FAST_EXEC_ROUNDS.len() - 1];
            for (chunk_idx, chunk) in uplift_candidates
                .chunks(STARVATION_LOCAL_UPLIFT_MAX_PENDING_SELECTORS)
                .take(STARVATION_LOCAL_UPLIFT_MAX_CHUNKS)
                .enumerate()
            {
                if pending.is_empty() {
                    break;
                }
                let target_set: HashSet<([u8; 4], usize)> = chunk.iter().copied().collect();
                let uplift_pending: Vec<([u8; 4], usize)> = pending
                    .iter()
                    .copied()
                    .filter(|item| target_set.contains(item))
                    .collect();
                if trace_starvation_local_uplift {
                    eprintln!(
                        "[trace-starvation-uplift] chunk={} selected={} chunk_size={}",
                        chunk_idx,
                        uplift_pending.len(),
                        chunk.len()
                    );
                }
                if uplift_pending.is_empty() {
                    continue;
                }

                let uplift = run_batch_round(
                    code,
                    uplift_pending,
                    allow_direct_entry,
                    &mut events,
                    &mut seen,
                    &may_reach_log,
                    path_index.as_ref(),
                    &mut static_dead_cache,
                    &mut probe_cache,
                    &mut entry_state_cache,
                    &mut jump_classify_cache,
                    &mut stats,
                    &mut profile,
                    gas_limit,
                    max_depth,
                    max_steps,
                    starvation_uplift_max_visited_states,
                );
                let unresolved_uplift: HashSet<([u8; 4], usize)> =
                    uplift.pending.into_iter().collect();
                pending
                    .retain(|item| !target_set.contains(item) || unresolved_uplift.contains(item));
            }
        } else if trace_starvation_local_uplift {
            eprintln!(
                "[trace-starvation-uplift] skip pending={} starved={} had_cap_hit={} disabled={}",
                pending.len(),
                starvation_pending.len(),
                had_visited_cap_hits,
                disable_starvation_local_uplift
            );
        }

        if !pending.is_empty()
            && should_run_recall_rounds(
                selectors.len(),
                pending.len(),
                seen.len(),
                static_candidates,
            )
        {
            let may_reach_all = vec![true; code.len()];
            let recall_rounds: &[(u32, u8, u32)] = if seen.is_empty() {
                &RECALL_EXEC_ROUNDS
            } else {
                &RECALL_EXEC_ROUNDS_PARTIAL
            };
            for &(gas_limit, max_depth, max_steps) in recall_rounds {
                if pending.is_empty() {
                    break;
                }
                pending = run_batch_round(
                    code,
                    pending,
                    allow_direct_entry,
                    &mut events,
                    &mut seen,
                    &may_reach_all,
                    path_index.as_ref(),
                    &mut static_dead_cache,
                    &mut probe_cache,
                    &mut entry_state_cache,
                    &mut jump_classify_cache,
                    &mut stats,
                    &mut profile,
                    gas_limit,
                    max_depth,
                    max_steps,
                    base_max_visited_states,
                )
                .pending;
            }
        }
    }
    if extra_fallback_round && has_any_selector {
        let calldata = CallDataImpl { selector: [0; 4] };
        let (gas_limit, max_depth, max_steps) = FAST_EXEC_ROUNDS[0];
        let may_reach_all = vec![true; code.len()];
        let _ = execute_from_entry(
            code,
            &calldata,
            &mut events,
            &mut seen,
            &may_reach_all,
            path_index.as_ref(),
            &mut static_dead_cache,
            &mut probe_cache,
            &mut jump_classify_cache,
            &mut stats,
            &mut profile,
            gas_limit,
            max_depth,
            max_steps,
            base_max_visited_states,
        );
    }

    let enable_static_supplement = std::env::var_os("EVMOLE_ENABLE_STATIC_SUPPLEMENT").is_some()
        && std::env::var_os("EVMOLE_DISABLE_STATIC_SUPPLEMENT").is_none();
    if enable_static_supplement {
        let break_on_jump = std::env::var_os("EVMOLE_STATIC_SUPPLEMENT_CROSS_JUMP").is_none();
        let window = static_supplement_window();
        for topic in static_event_candidate_set(code, window, break_on_jump) {
            collect_event(&mut events, &mut seen, topic);
        }
    }

    events.sort_unstable();
    (events, stats)
}

pub fn contract_events_with_stats(code: &[u8]) -> (Vec<EventSelector>, EventExtractionStats) {
    contract_events_with_stats_internal(code, None)
}

pub fn contract_events_with_profile(
    code: &[u8],
) -> (
    Vec<EventSelector>,
    EventExtractionStats,
    EventExecutionProfile,
) {
    let mut profile = EventExecutionProfile::default();
    let (events, stats) = contract_events_with_stats_internal(code, Some(&mut profile));
    (events, stats, profile)
}

/// Extracts all event selectors from contract bytecode.
pub fn contract_events(code: &[u8]) -> Vec<EventSelector> {
    contract_events_with_stats(code).0
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

    #[test]
    fn test_static_event_candidates_from_log_long_distance() {
        let topic = [0x7a; 32];
        let mut code = Vec::new();
        code.push(op::PUSH32);
        code.extend_from_slice(&topic);
        for _ in 0..80 {
            code.extend_from_slice(&[op::PUSH1, 0x00, op::POP]);
        }
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1, op::STOP]);

        let short_window = static_event_candidate_set(&code, 24, true);
        assert!(!short_window.contains(&topic));

        let long_window = static_event_candidate_set(&code, 192, true);
        assert!(long_window.contains(&topic));
    }

    #[test]
    fn test_static_event_candidates_cross_inline_invalid_guard() {
        let topic = [0x6b; 32];
        let mut code = Vec::new();
        code.push(op::PUSH32);
        code.extend_from_slice(&topic);

        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x01]);
        let jump_dest_patch = code.len() - 3;
        code.push(op::JUMPI);
        code.push(op::INVALID);
        let jump_dest = code.len();
        code.push(op::JUMPDEST);
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1, op::STOP]);
        code[jump_dest_patch] = u8::try_from(jump_dest).unwrap();

        let candidates = static_event_candidate_set(&code, 64, true);
        assert!(candidates.contains(&topic));
    }

    #[test]
    fn test_static_dead_end_follows_push_jump() {
        let code = vec![
            op::JUMPDEST,
            op::PUSH2,
            0x00,
            0x08,
            op::JUMP,
            op::STOP,
            op::STOP,
            op::STOP,
            op::JUMPDEST,
            op::PUSH1,
            0x00,
            op::PUSH1,
            0x00,
            op::REVERT,
        ];

        assert!(is_static_dead_end(&code, 0));
        assert!(is_static_dead_end(&code, 8));
        assert!(!is_static_dead_end(&code, 5));
    }

    #[test]
    fn test_static_dead_end_allows_revert_prelude_ops() {
        let code = vec![
            op::JUMPDEST,
            op::PUSH1,
            0x00,
            op::PUSH1,
            0x00,
            op::MSTORE,
            op::CALLDATASIZE,
            op::ISZERO,
            op::PUSH1,
            0x00,
            op::PUSH1,
            0x00,
            op::REVERT,
        ];

        assert!(is_static_dead_end(&code, 0));
    }

    #[test]
    fn test_may_reach_log_respects_static_jump_targets() {
        let selector = [0x99; 32];
        let mut code = vec![op::PUSH1, 0x00, op::JUMP];

        let dead_block = code.len();
        code.push(op::JUMPDEST);
        code.extend_from_slice(&[op::PUSH1, 0x00]);
        let dead_target_patch = code.len() - 1;
        code.push(op::JUMP);

        let log_block = code.len();
        code[1] = u8::try_from(log_block).unwrap();
        code[dead_target_patch] = u8::try_from(dead_block).unwrap();

        code.push(op::JUMPDEST);
        append_log1(&mut code, selector);
        code.push(op::STOP);

        let may_reach = compute_may_reach_log(&code);
        assert!(may_reach[2]); // static jump to LOG block
        assert!(!may_reach[dead_block]); // disconnected self-loop block
        assert!(!may_reach[dead_block + 3]); // jump inside disconnected block
    }

    #[test]
    fn test_safe_direct_entry_rejects_pop_prologue() {
        let code = vec![op::JUMPDEST, op::POP, op::STOP];
        assert!(!is_safe_direct_function_entry(&code, 0));
    }

    #[test]
    fn test_safe_direct_entry_accepts_stack_neutral_prologue() {
        let code = vec![
            op::JUMPDEST,
            op::CALLVALUE,
            op::ISZERO,
            op::PUSH1,
            0x00,
            op::JUMPI,
        ];
        assert!(is_safe_direct_function_entry(&code, 0));
    }
}
