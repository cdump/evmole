use std::{collections::BTreeMap, collections::VecDeque};

use crate::collections::{HashMap, HashSet};
use crate::control_flow_graph::{
    Block, BlockType, INVALID_JUMP_START, basic_blocks, control_flow_graph,
    state::{StackSym, State},
};
use crate::evm::{code_iterator::iterate_code, op};
use crate::selectors::function_selectors;

use super::{EventSelector, is_plausible_event_hash};

const DEFAULT_MAX_STATES_PER_LOG: usize = 20_000;
const DEFAULT_MAX_PRED_STEPS_PER_LOG: usize = 20_000;

#[derive(Clone, Copy)]
struct BottomUpConfig {
    max_states_per_log: usize,
    max_pred_steps_per_log: usize,
}

impl BottomUpConfig {
    fn from_env() -> Self {
        let max_states_per_log = std::env::var("EVMOLE_EVENTS_BOTTOM_UP_MAX_STATES")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_MAX_STATES_PER_LOG);
        let max_pred_steps_per_log = std::env::var("EVMOLE_EVENTS_BOTTOM_UP_MAX_PRED_STEPS")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_MAX_PRED_STEPS_PER_LOG);
        Self {
            max_states_per_log,
            max_pred_steps_per_log,
        }
    }
}

#[derive(Clone, Copy)]
struct LogSite {
    pc: usize,
    block_start: usize,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct BacktrackKey {
    block_start: usize,
    context: usize,
    sym: StackSym,
}

#[derive(Clone)]
struct BacktrackState {
    block_start: usize,
    context: usize,
    sym: StackSym,
}

struct CfgIndex {
    blocks: BTreeMap<usize, Block>,
    preds_by_block: HashMap<usize, HashSet<usize>>,
    contexts_reaching_block: HashMap<usize, HashSet<usize>>,
}

pub(crate) fn contract_events_bottom_up(code: &[u8]) -> Vec<EventSelector> {
    if code.is_empty() {
        return Vec::new();
    }
    let cfg = BottomUpConfig::from_env();
    let index = build_cfg_index(code);
    if index.blocks.is_empty() {
        return Vec::new();
    }
    let log_sites = collect_log_sites(code, &index.blocks);
    if log_sites.is_empty() {
        return Vec::new();
    }

    let mut out: HashSet<EventSelector> = HashSet::default();
    let trace = std::env::var_os("EVMOLE_EVENTS_BOTTOM_UP_TRACE").is_some();
    for log_site in log_sites {
        resolve_log_site(code, &index, log_site, cfg, &mut out, trace);
    }

    let mut events: Vec<EventSelector> = out.into_iter().collect();
    events.sort_unstable();
    events
}

fn build_cfg_index(code: &[u8]) -> CfgIndex {
    let cfg = control_flow_graph(code, basic_blocks(code));

    let mut succ_by_block: HashMap<usize, HashSet<usize>> = HashMap::default();
    let mut preds_by_block: HashMap<usize, HashSet<usize>> = HashMap::default();

    let mut add_edge = |from: usize, to: usize| {
        if to >= INVALID_JUMP_START || !cfg.blocks.contains_key(&to) {
            return;
        }
        succ_by_block.entry(from).or_default().insert(to);
        preds_by_block.entry(to).or_default().insert(from);
    };

    for (start, block) in &cfg.blocks {
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

    let contexts = collect_contexts(code);
    let mut contexts_reaching_block: HashMap<usize, HashSet<usize>> = HashMap::default();
    for context in contexts {
        let Some(entry) = find_block_start(&cfg.blocks, context) else {
            continue;
        };
        let mut stack = vec![entry];
        let mut seen: HashSet<usize> = HashSet::default();
        while let Some(block) = stack.pop() {
            if !seen.insert(block) {
                continue;
            }
            contexts_reaching_block
                .entry(block)
                .or_default()
                .insert(context);
            if let Some(nexts) = succ_by_block.get(&block) {
                stack.extend(nexts.iter().copied());
            }
        }
    }

    CfgIndex {
        blocks: cfg.blocks,
        preds_by_block,
        contexts_reaching_block,
    }
}

fn collect_contexts(code: &[u8]) -> Vec<usize> {
    let (selectors, _) = function_selectors(code, 0);
    let mut set: HashSet<usize> = HashSet::default();
    // Include PC 0 to cover fallback/receive paths.
    set.insert(0);
    set.extend(selectors.into_values());
    let mut out: Vec<usize> = set.into_iter().collect();
    out.sort_unstable();
    out
}

fn collect_log_sites(code: &[u8], blocks: &BTreeMap<usize, Block>) -> Vec<LogSite> {
    let mut out = Vec::new();
    for (start, block) in blocks {
        for (pc, cop) in iterate_code(code, *start, Some(block.end)) {
            if (op::LOG1..=op::LOG4).contains(&cop.op) {
                out.push(LogSite {
                    pc,
                    block_start: *start,
                });
            }
        }
    }
    out
}

fn resolve_log_site(
    code: &[u8],
    index: &CfgIndex,
    log_site: LogSite,
    cfg: BottomUpConfig,
    out: &mut HashSet<EventSelector>,
    trace: bool,
) {
    let Some(init_sym) = topic0_symbol_at_log(code, &index.blocks, log_site) else {
        if trace {
            eprintln!(
                "[bu-trace] log_pc=0x{:x} block=0x{:x} init_sym=<none>",
                log_site.pc, log_site.block_start
            );
        }
        return;
    };
    let Some(contexts) = index.contexts_reaching_block.get(&log_site.block_start) else {
        if trace {
            eprintln!(
                "[bu-trace] log_pc=0x{:x} block=0x{:x} init_sym={:?} contexts=0 (skip)",
                log_site.pc, log_site.block_start, init_sym
            );
        }
        return;
    };
    if trace {
        eprintln!(
            "[bu-trace] log_pc=0x{:x} block=0x{:x} init_sym={:?} contexts={}",
            log_site.pc,
            log_site.block_start,
            init_sym,
            contexts.len()
        );
    }

    let mut queue: VecDeque<BacktrackState> = VecDeque::new();
    queue.extend(contexts.iter().copied().map(|context| BacktrackState {
        block_start: log_site.block_start,
        context,
        sym: init_sym.clone(),
    }));

    let mut visited: HashSet<BacktrackKey> = HashSet::default();
    let mut state_cache: HashMap<usize, State> = HashMap::default();
    let mut processed_states = 0usize;
    let mut pred_steps = 0usize;
    let before_count = out.len();
    let mut first_unresolved_other: Option<usize> = None;

    while let Some(state) = queue.pop_front() {
        if processed_states >= cfg.max_states_per_log {
            break;
        }
        processed_states += 1;

        let key = BacktrackKey {
            block_start: state.block_start,
            context: state.context,
            sym: state.sym.clone(),
        };
        if !visited.insert(key) {
            continue;
        }

        match state.sym {
            StackSym::Other(pc) => {
                if let Some(topic) = push32_value(code, pc)
                    && is_plausible_event_hash(&topic)
                {
                    out.insert(topic);
                } else if first_unresolved_other.is_none() {
                    first_unresolved_other = Some(pc);
                }
            }
            StackSym::Before(n) => {
                let Some(preds) = index.preds_by_block.get(&state.block_start) else {
                    continue;
                };
                for pred in preds {
                    if pred_steps >= cfg.max_pred_steps_per_log {
                        break;
                    }
                    pred_steps += 1;

                    let context_reaches_pred = index
                        .contexts_reaching_block
                        .get(pred)
                        .is_some_and(|set| set.contains(&state.context));
                    if !context_reaches_pred {
                        continue;
                    }

                    let Some(exit_sym) =
                        block_exit_symbol_at_slot(code, &index.blocks, *pred, n, &mut state_cache)
                    else {
                        continue;
                    };
                    queue.push_back(BacktrackState {
                        block_start: *pred,
                        context: state.context,
                        sym: exit_sym,
                    });
                }
            }
            StackSym::Pushed(_) | StackSym::Jumpdest(_) => {}
        }
    }
    if trace {
        let produced = out.len().saturating_sub(before_count);
        eprintln!(
            "[bu-trace] log_pc=0x{:x} produced={} states={} pred_steps={} unresolved_other_pc={}",
            log_site.pc,
            produced,
            processed_states,
            pred_steps,
            first_unresolved_other
                .map(|pc| format!("0x{pc:x}"))
                .unwrap_or_else(|| "-".to_string())
        );
    }
}

fn topic0_symbol_at_log(
    code: &[u8],
    blocks: &BTreeMap<usize, Block>,
    log_site: LogSite,
) -> Option<StackSym> {
    let block = blocks.get(&log_site.block_start)?;
    let mut state = State::new();
    // LOG pops topic0, so execute until the instruction before LOG.
    if let Some(prev_pc) = find_prev_instruction_pc(code, block.start, log_site.pc) {
        let _ = state.exec(code, block.start, Some(prev_pc));
    }
    Some(state.get_stack(2))
}

fn block_exit_symbol_at_slot(
    code: &[u8],
    blocks: &BTreeMap<usize, Block>,
    block_start: usize,
    slot: usize,
    state_cache: &mut HashMap<usize, State>,
) -> Option<StackSym> {
    if !state_cache.contains_key(&block_start) {
        let block = blocks.get(&block_start)?;
        let mut state = State::new();
        let _ = state.exec(code, block.start, Some(block.end));
        state_cache.insert(block_start, state);
    }
    state_cache
        .get(&block_start)
        .map(|state| state.get_stack(slot))
}

fn find_prev_instruction_pc(code: &[u8], start_pc: usize, target_pc: usize) -> Option<usize> {
    let mut prev = None;
    for (pc, _) in iterate_code(code, start_pc, Some(target_pc)) {
        if pc == target_pc {
            return prev;
        }
        prev = Some(pc);
    }
    None
}

fn find_block_start(blocks: &BTreeMap<usize, Block>, pc: usize) -> Option<usize> {
    let (start, block) = blocks.range(..=pc).next_back()?;
    if pc <= block.end { Some(*start) } else { None }
}

fn push32_value(code: &[u8], pc: usize) -> Option<EventSelector> {
    if code.get(pc).copied()? != op::PUSH32 {
        return None;
    }
    let end = pc.checked_add(33)?;
    if end > code.len() {
        return None;
    }
    let mut topic = [0u8; 32];
    topic.copy_from_slice(&code[pc + 1..end]);
    Some(topic)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn append_log1(code: &mut Vec<u8>, selector: [u8; 32]) {
        code.push(op::PUSH32);
        code.extend_from_slice(&selector);
        code.extend_from_slice(&[op::PUSH1, 0x00, op::PUSH1, 0x00, op::LOG1]);
    }

    #[test]
    fn bottom_up_simple_log1() {
        let selector = [0xabu8; 32];
        let mut code = Vec::new();
        append_log1(&mut code, selector);
        code.push(op::STOP);

        let events = contract_events_bottom_up(&code);
        assert_eq!(events, vec![selector]);
    }

    #[test]
    fn bottom_up_cross_block_before_chain() {
        let selector = [0x11u8; 32];
        let mut code = Vec::new();
        code.push(op::PUSH32);
        code.extend_from_slice(&selector);
        code.extend_from_slice(&[
            op::PUSH1,
            0x24, // jumpdest at pc 0x24
            op::JUMP,
            op::JUMPDEST,
            op::PUSH1,
            0x00,
            op::PUSH1,
            0x00,
            op::LOG1,
            op::STOP,
        ]);

        let events = contract_events_bottom_up(&code);
        assert_eq!(events, vec![selector]);
    }
}
