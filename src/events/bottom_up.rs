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
const BLOCK_STATE_CACHE_MAX_ENTRIES: usize = 2_048;
const HOTLOG_MAX_LOGS: usize = 8;
const HOTLOG_BOOST_MAX_STATES_PER_LOG: usize = 100_000;
const HOTLOG_BOOST_MAX_PRED_STEPS_PER_LOG: usize = 100_000;
const HOTLOG_TOTAL_STATES_BUDGET: usize = 500_000;
const HOTLOG_TOTAL_PRED_STEPS_BUDGET: usize = 500_000;
const HOTLOG_EARLY_ABORT_STATES: usize = 30_000;
const HOTLOG_MAX_ZERO_GAIN_STREAK: usize = 2;

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

#[derive(Clone, Hash, PartialEq, Eq)]
struct HotlogBacktrackKey {
    block_start: usize,
    sym: StackSym,
}

#[derive(Clone)]
struct HotlogBacktrackState {
    block_start: usize,
    contexts: HashSet<usize>,
    sym: StackSym,
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum HotlogCandidateKey {
    UnresolvedOther(usize),
    BlockSym { block_start: usize, sym: StackSym },
}

struct CfgIndex {
    blocks: BTreeMap<usize, Block>,
    preds_by_block: HashMap<usize, HashSet<usize>>,
    contexts_reaching_block: HashMap<usize, HashSet<usize>>,
}

#[derive(Clone, Default)]
struct LogResolveStats {
    init_sym: Option<StackSym>,
    direct_push32: bool,
    produced: usize,
    processed_states: usize,
    pred_steps: usize,
    hit_states_cap: bool,
    hit_pred_cap: bool,
    first_unresolved_other: Option<usize>,
}

struct BlockStateCache {
    map: HashMap<usize, State>,
    insertion_order: VecDeque<usize>,
    max_entries: usize,
}

impl BlockStateCache {
    fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::default(),
            insertion_order: VecDeque::new(),
            max_entries,
        }
    }

    fn insert(&mut self, block_start: usize, state: State) {
        if self.max_entries == 0 {
            return;
        }
        if self.map.contains_key(&block_start) {
            return;
        }
        self.map.insert(block_start, state);
        self.insertion_order.push_back(block_start);
        while self.map.len() > self.max_entries {
            let Some(old) = self.insertion_order.pop_front() else {
                break;
            };
            if self.map.remove(&old).is_some() {
                break;
            }
        }
    }

    fn get_exit_symbol(
        &mut self,
        code: &[u8],
        blocks: &BTreeMap<usize, Block>,
        block_start: usize,
        slot: usize,
    ) -> Option<StackSym> {
        if !self.map.contains_key(&block_start) {
            let block = blocks.get(&block_start)?;
            let mut state = State::new();
            let _ = state.exec(code, block.start, Some(block.end));
            self.insert(block_start, state);
        }
        self.map
            .get(&block_start)
            .map(|state| state.get_stack(slot))
    }
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
    let hotlog_enabled = std::env::var_os("EVMOLE_EVENTS_BOTTOM_UP_HOTLOG").is_some();
    let hotlog_trace = std::env::var_os("EVMOLE_EVENTS_BOTTOM_UP_HOTLOG_TRACE").is_some();
    let mut state_cache = BlockStateCache::new(BLOCK_STATE_CACHE_MAX_ENTRIES);

    let mut first_pass: Vec<(LogSite, LogResolveStats)> = Vec::with_capacity(log_sites.len());
    for log_site in log_sites.iter().copied() {
        let stats = resolve_log_site(
            code,
            &index,
            log_site,
            cfg,
            &mut out,
            &mut state_cache,
            trace,
        );
        first_pass.push((log_site, stats));
    }

    if hotlog_enabled {
        let hot_cfg = BottomUpConfig {
            max_states_per_log: HOTLOG_BOOST_MAX_STATES_PER_LOG.max(cfg.max_states_per_log),
            max_pred_steps_per_log: HOTLOG_BOOST_MAX_PRED_STEPS_PER_LOG
                .max(cfg.max_pred_steps_per_log),
        };
        let mut candidates: Vec<usize> = first_pass
            .iter()
            .enumerate()
            .filter_map(|(idx, (_, stats))| should_run_hotlog_second_pass(stats).then_some(idx))
            .collect();
        candidates.sort_by(|a, b| {
            first_pass[*b]
                .1
                .pred_steps
                .cmp(&first_pass[*a].1.pred_steps)
                .then_with(|| first_pass[*a].0.pc.cmp(&first_pass[*b].0.pc))
        });
        let candidates_before_dedup = candidates.len();
        let mut seen_log_keys: HashSet<HotlogCandidateKey> = HashSet::default();
        candidates.retain(|idx| {
            let (site, stats) = &first_pass[*idx];
            let Some(sym) = &stats.init_sym else {
                return false;
            };
            let key = if let Some(pc) = stats.first_unresolved_other {
                HotlogCandidateKey::UnresolvedOther(pc)
            } else {
                HotlogCandidateKey::BlockSym {
                    block_start: site.block_start,
                    sym: sym.clone(),
                }
            };
            seen_log_keys.insert(key)
        });

        if hotlog_trace {
            eprintln!(
                "[bu-hotlog] candidates={} deduped={} selected={} states_budget={} pred_budget={}",
                candidates_before_dedup,
                candidates.len(),
                candidates.len().min(HOTLOG_MAX_LOGS),
                HOTLOG_TOTAL_STATES_BUDGET,
                HOTLOG_TOTAL_PRED_STEPS_BUDGET
            );
        }

        let mut extra_states_used = 0usize;
        let mut extra_pred_steps_used = 0usize;
        let mut zero_gain_streak = 0usize;
        for idx in candidates.into_iter().take(HOTLOG_MAX_LOGS) {
            if extra_states_used >= HOTLOG_TOTAL_STATES_BUDGET
                || extra_pred_steps_used >= HOTLOG_TOTAL_PRED_STEPS_BUDGET
            {
                break;
            }
            let states_left = HOTLOG_TOTAL_STATES_BUDGET.saturating_sub(extra_states_used);
            let pred_left = HOTLOG_TOTAL_PRED_STEPS_BUDGET.saturating_sub(extra_pred_steps_used);
            if states_left == 0 || pred_left == 0 {
                break;
            }
            let pass_cfg = BottomUpConfig {
                max_states_per_log: hot_cfg.max_states_per_log.min(states_left),
                max_pred_steps_per_log: hot_cfg.max_pred_steps_per_log.min(pred_left),
            };
            let log_site = first_pass[idx].0;
            let before = out.len();
            let stats = resolve_log_site_hotlog(
                code,
                &index,
                log_site,
                pass_cfg,
                &mut out,
                &mut state_cache,
                hotlog_trace,
            );
            extra_states_used = extra_states_used.saturating_add(stats.processed_states);
            extra_pred_steps_used = extra_pred_steps_used.saturating_add(stats.pred_steps);
            let gained = out.len().saturating_sub(before);
            if gained == 0 {
                zero_gain_streak = zero_gain_streak.saturating_add(1);
            } else {
                zero_gain_streak = 0;
            }

            if hotlog_trace {
                let first = &first_pass[idx].1;
                eprintln!(
                    "[bu-hotlog] log_pc=0x{:x} first(states={} pred_steps={} cap_s={} cap_p={} produced={}) second(states={} pred_steps={} cap_s={} cap_p={} gained={})",
                    log_site.pc,
                    first.processed_states,
                    first.pred_steps,
                    first.hit_states_cap,
                    first.hit_pred_cap,
                    first.produced,
                    stats.processed_states,
                    stats.pred_steps,
                    stats.hit_states_cap,
                    stats.hit_pred_cap,
                    gained
                );
            }

            if zero_gain_streak >= HOTLOG_MAX_ZERO_GAIN_STREAK {
                if hotlog_trace {
                    eprintln!(
                        "[bu-hotlog] stop zero_gain_streak={} threshold={}",
                        zero_gain_streak, HOTLOG_MAX_ZERO_GAIN_STREAK
                    );
                }
                break;
            }
        }
        if hotlog_trace {
            eprintln!(
                "[bu-hotlog] used_states={} used_pred_steps={} cache_size={}",
                extra_states_used,
                extra_pred_steps_used,
                state_cache.map.len()
            );
        }
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
    state_cache: &mut BlockStateCache,
    trace: bool,
) -> LogResolveStats {
    let mut stats = LogResolveStats::default();
    let Some(init_sym) = topic0_symbol_at_log(code, &index.blocks, log_site) else {
        stats.hit_states_cap = false;
        stats.hit_pred_cap = false;
        if trace {
            eprintln!(
                "[bu-trace] log_pc=0x{:x} block=0x{:x} init_sym=<none>",
                log_site.pc, log_site.block_start
            );
        }
        return stats;
    };
    stats.direct_push32 = is_direct_push32_sym(code, &init_sym);
    stats.init_sym = Some(init_sym.clone());

    let Some(contexts) = index.contexts_reaching_block.get(&log_site.block_start) else {
        if trace {
            eprintln!(
                "[bu-trace] log_pc=0x{:x} block=0x{:x} init_sym={:?} contexts=0 (skip)",
                log_site.pc, log_site.block_start, init_sym
            );
        }
        return stats;
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
    let mut processed_states = 0usize;
    let mut pred_steps = 0usize;
    let before_count = out.len();
    let mut first_unresolved_other: Option<usize> = None;

    while let Some(state) = queue.pop_front() {
        if processed_states >= cfg.max_states_per_log {
            stats.hit_states_cap = true;
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
                        stats.hit_pred_cap = true;
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
                        block_exit_symbol_at_slot(code, &index.blocks, *pred, n, state_cache)
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
    stats.produced = out.len().saturating_sub(before_count);
    stats.processed_states = processed_states;
    stats.pred_steps = pred_steps;
    stats.first_unresolved_other = first_unresolved_other;

    if trace {
        eprintln!(
            "[bu-trace] log_pc=0x{:x} produced={} states={} pred_steps={} cap_s={} cap_p={} unresolved_other_pc={}",
            log_site.pc,
            stats.produced,
            stats.processed_states,
            stats.pred_steps,
            stats.hit_states_cap,
            stats.hit_pred_cap,
            first_unresolved_other
                .map(|pc| format!("0x{pc:x}"))
                .unwrap_or_else(|| "-".to_string())
        );
    }
    stats
}

fn resolve_log_site_hotlog(
    code: &[u8],
    index: &CfgIndex,
    log_site: LogSite,
    cfg: BottomUpConfig,
    out: &mut HashSet<EventSelector>,
    state_cache: &mut BlockStateCache,
    trace: bool,
) -> LogResolveStats {
    let mut stats = LogResolveStats::default();
    let Some(init_sym) = topic0_symbol_at_log(code, &index.blocks, log_site) else {
        if trace {
            eprintln!(
                "[bu-hotlog-trace] log_pc=0x{:x} block=0x{:x} init_sym=<none>",
                log_site.pc, log_site.block_start
            );
        }
        return stats;
    };
    stats.direct_push32 = is_direct_push32_sym(code, &init_sym);
    stats.init_sym = Some(init_sym.clone());

    let Some(contexts) = index.contexts_reaching_block.get(&log_site.block_start) else {
        if trace {
            eprintln!(
                "[bu-hotlog-trace] log_pc=0x{:x} block=0x{:x} init_sym={:?} contexts=0 (skip)",
                log_site.pc, log_site.block_start, init_sym
            );
        }
        return stats;
    };

    let mut queue: VecDeque<HotlogBacktrackState> = VecDeque::new();
    queue.push_back(HotlogBacktrackState {
        block_start: log_site.block_start,
        contexts: contexts.clone(),
        sym: init_sym,
    });

    let mut visited_contexts: HashMap<HotlogBacktrackKey, HashSet<usize>> = HashMap::default();
    let mut processed_states = 0usize;
    let mut pred_steps = 0usize;
    let before_count = out.len();
    let mut first_unresolved_other: Option<usize> = None;

    while let Some(state) = queue.pop_front() {
        if processed_states >= cfg.max_states_per_log {
            stats.hit_states_cap = true;
            break;
        }

        let key = HotlogBacktrackKey {
            block_start: state.block_start,
            sym: state.sym.clone(),
        };
        let mut active_contexts = state.contexts;
        if let Some(seen) = visited_contexts.get_mut(&key) {
            active_contexts.retain(|ctx| !seen.contains(ctx));
            if active_contexts.is_empty() {
                continue;
            }
            seen.extend(active_contexts.iter().copied());
        } else {
            visited_contexts.insert(key, active_contexts.clone());
        }
        processed_states += 1;
        if processed_states >= HOTLOG_EARLY_ABORT_STATES && out.len() == before_count {
            if trace {
                eprintln!(
                    "[bu-hotlog-trace] log_pc=0x{:x} early_abort states={} no_gain=true",
                    log_site.pc, processed_states
                );
            }
            break;
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
                        stats.hit_pred_cap = true;
                        break;
                    }
                    pred_steps += 1;

                    let Some(pred_contexts) = index.contexts_reaching_block.get(pred) else {
                        continue;
                    };
                    let next_contexts: HashSet<usize> = active_contexts
                        .iter()
                        .copied()
                        .filter(|ctx| pred_contexts.contains(ctx))
                        .collect();
                    if next_contexts.is_empty() {
                        continue;
                    }

                    let Some(exit_sym) =
                        block_exit_symbol_at_slot(code, &index.blocks, *pred, n, state_cache)
                    else {
                        continue;
                    };
                    queue.push_back(HotlogBacktrackState {
                        block_start: *pred,
                        contexts: next_contexts,
                        sym: exit_sym,
                    });
                }
            }
            StackSym::Pushed(_) | StackSym::Jumpdest(_) => {}
        }
    }
    stats.produced = out.len().saturating_sub(before_count);
    stats.processed_states = processed_states;
    stats.pred_steps = pred_steps;
    stats.first_unresolved_other = first_unresolved_other;

    if trace {
        eprintln!(
            "[bu-hotlog-trace] log_pc=0x{:x} produced={} states={} pred_steps={} cap_s={} cap_p={} unresolved_other_pc={}",
            log_site.pc,
            stats.produced,
            stats.processed_states,
            stats.pred_steps,
            stats.hit_states_cap,
            stats.hit_pred_cap,
            first_unresolved_other
                .map(|pc| format!("0x{pc:x}"))
                .unwrap_or_else(|| "-".to_string())
        );
    }
    stats
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
    state_cache: &mut BlockStateCache,
) -> Option<StackSym> {
    state_cache.get_exit_symbol(code, blocks, block_start, slot)
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

fn is_direct_push32_sym(code: &[u8], sym: &StackSym) -> bool {
    match sym {
        StackSym::Other(pc) => push32_value(code, *pc).is_some(),
        _ => false,
    }
}

fn should_run_hotlog_second_pass(stats: &LogResolveStats) -> bool {
    stats.init_sym.is_some() && (stats.hit_states_cap || stats.hit_pred_cap) && !stats.direct_push32
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
