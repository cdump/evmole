use std::collections::VecDeque;

use crate::collections::{HashMap, HashSet};
use crate::control_flow_graph::state::{StackSym, State};
use crate::evm::{code_iterator::iterate_code, op};

use super::classify::{
    CfgIndex, ClassifiedLogSite, LogSiteClass, find_block_start, find_prev_instruction_pc,
};
use super::{EventSelector, is_plausible_event_hash};

const MAX_STATES_PER_LOG: usize = 500_000;
const MAX_PRED_STEPS_PER_LOG: usize = 500_000;
const BLOCK_STATE_CACHE_MAX_ENTRIES: usize = 2_048;
const CONTINUATION_CACHE_MAX_ENTRIES: usize = 4_096;

// ---------------------------------------------------------------------------
// Block state cache (LRU-ish, shared across all LOG sites)
// ---------------------------------------------------------------------------

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
        if self.max_entries == 0 || self.map.contains_key(&block_start) {
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
        index: &CfgIndex,
        block_start: usize,
        slot: usize,
    ) -> Option<StackSym> {
        if !self.map.contains_key(&block_start) {
            let block = index.blocks.get(&block_start)?;
            let mut state = State::new();
            let _ = state.exec(code, block.start, Some(block.end));
            self.insert(block_start, state);
        }
        self.map
            .get(&block_start)
            .map(|state| state.get_stack(slot))
    }
}

// ---------------------------------------------------------------------------
// Continuation cache (shared across all LOG sites)
// ---------------------------------------------------------------------------

struct ContinuationCache {
    // Key: (block_start, slot) → Value: [(pred_block, exit_sym)]
    map: HashMap<(usize, usize), Vec<(usize, StackSym)>>,
    insertion_order: VecDeque<(usize, usize)>,
    max_entries: usize,
}

impl ContinuationCache {
    fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::default(),
            insertion_order: VecDeque::new(),
            max_entries,
        }
    }

    fn get_or_compute(
        &mut self,
        code: &[u8],
        index: &CfgIndex,
        state_cache: &mut BlockStateCache,
        block_start: usize,
        slot: usize,
    ) -> &[(usize, StackSym)] {
        let key = (block_start, slot);
        if !self.map.contains_key(&key) {
            let result = Self::compute(code, index, state_cache, block_start, slot);
            self.insert(key, result);
        }
        self.map.get(&key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    fn compute(
        code: &[u8],
        index: &CfgIndex,
        state_cache: &mut BlockStateCache,
        block_start: usize,
        slot: usize,
    ) -> Vec<(usize, StackSym)> {
        let Some(preds) = index.preds_by_block.get(&block_start) else {
            return Vec::new();
        };
        preds
            .iter()
            .filter_map(|&pred| {
                state_cache
                    .get_exit_symbol(code, index, pred, slot)
                    .map(|sym| (pred, sym))
            })
            .collect()
    }

    fn insert(&mut self, key: (usize, usize), result: Vec<(usize, StackSym)>) {
        if self.max_entries == 0 || self.map.contains_key(&key) {
            return;
        }
        self.map.insert(key, result);
        self.insertion_order.push_back(key);
        while self.map.len() > self.max_entries {
            let Some(old) = self.insertion_order.pop_front() else {
                break;
            };
            if self.map.remove(&old).is_some() {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Backtrack types (BFS dedup)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Dispatch entry
// ---------------------------------------------------------------------------

pub(super) fn resolve_classified_log_sites(
    code: &[u8],
    index: &CfgIndex,
    sites: &[ClassifiedLogSite],
) -> Vec<EventSelector> {
    let mut out: HashSet<EventSelector> = HashSet::default();
    let mut state_cache = BlockStateCache::new(BLOCK_STATE_CACHE_MAX_ENTRIES);
    let mut cont_cache = ContinuationCache::new(CONTINUATION_CACHE_MAX_ENTRIES);

    for site in sites {
        match site.class {
            LogSiteClass::Push32 { topic_pc } => {
                resolve_push32(code, topic_pc, &mut out);
            }
            LogSiteClass::PushN { topic_pc } => {
                resolve_pushn(code, topic_pc, &mut out);
            }
            LogSiteClass::MloadCodecopy { mload_pc } => {
                resolve_mload_codecopy(code, mload_pc, site.site.block_start, &mut out);
            }
            LogSiteClass::CrossBlock { init_sym_n } => {
                resolve_cross_block(
                    code,
                    index,
                    site,
                    init_sym_n,
                    &mut state_cache,
                    &mut cont_cache,
                    &mut out,
                );
            }
        }
    }

    let mut events: Vec<EventSelector> = out.into_iter().collect();
    events.sort_unstable();
    events
}

// ---------------------------------------------------------------------------
// Sub-class a: PUSH32
// ---------------------------------------------------------------------------

fn resolve_push32(code: &[u8], pc: usize, out: &mut HashSet<EventSelector>) {
    if let Some(topic) = push32_value(code, pc) {
        out.insert(topic);
    }
}

fn push32_value(code: &[u8], pc: usize) -> Option<[u8; 32]> {
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

// ---------------------------------------------------------------------------
// Sub-class b: PUSH5..PUSH31 (right-aligned into 32 bytes)
// ---------------------------------------------------------------------------

fn resolve_pushn(code: &[u8], pc: usize, out: &mut HashSet<EventSelector>) {
    if let Some(topic) = pushn_value(code, pc) {
        out.insert(topic);
    }
}

fn pushn_value(code: &[u8], pc: usize) -> Option<[u8; 32]> {
    let opcode = *code.get(pc)?;
    if !(op::PUSH1..op::PUSH32).contains(&opcode) {
        return None;
    }
    let n = (opcode - op::PUSH1 + 1) as usize;
    let start = pc + 1;
    let end = start.checked_add(n)?;
    if end > code.len() {
        return None;
    }
    let mut topic = [0u8; 32];
    topic[32 - n..].copy_from_slice(&code[start..end]);
    Some(topic)
}

// ---------------------------------------------------------------------------
// Sub-class c: MLOAD preceded by CODECOPY
// ---------------------------------------------------------------------------

fn resolve_mload_codecopy(
    code: &[u8],
    mload_pc: usize,
    block_start: usize,
    out: &mut HashSet<EventSelector>,
) {
    if let Some(topic) = mload_codecopy_value(code, mload_pc, block_start) {
        out.insert(topic);
    }
}

fn mload_codecopy_value(code: &[u8], mload_pc: usize, block_start: usize) -> Option<[u8; 32]> {
    // Use symbolic execution to precisely identify CODECOPY's `offset` argument.
    // CODECOPY pops (destOffset, offset, size) from stack.
    let instrs: Vec<(usize, u8)> = iterate_code(code, block_start, Some(mload_pc))
        .map(|(pc, cop)| (pc, cop.op))
        .collect();

    // Find the last CODECOPY before the MLOAD.
    let (codecopy_pc, _) = *instrs.iter().rev().find(|&&(_, op)| op == op::CODECOPY)?;

    // Run symbolic execution up to (but not including) CODECOPY to read its arguments.
    let prev_pc = find_prev_instruction_pc(code, block_start, codecopy_pc)?;
    let mut state = State::new();
    let _ = state.exec(code, block_start, Some(prev_pc));

    // Helper: extract a concrete usize from a stack symbol.
    let sym_to_usize = |sym: &StackSym| -> Option<usize> {
        match sym {
            // PUSH1..PUSH4 → Pushed([u8; 4]) with value stored big-endian
            StackSym::Pushed(bytes) => Some(u32::from_be_bytes(*bytes) as usize),
            // PUSH5..PUSH32 → Other(pc), read concrete value from bytecode
            StackSym::Other(pc) => {
                let opcode = *code.get(*pc)?;
                if !(op::PUSH1..=op::PUSH32).contains(&opcode) {
                    return None;
                }
                let n = (opcode - op::PUSH1 + 1) as usize;
                let arg_start = pc + 1;
                let arg_end = arg_start.checked_add(n)?;
                if arg_end > code.len() {
                    return None;
                }
                let mut buf = [0u8; 8];
                let copy_start = 8usize.saturating_sub(n);
                buf[copy_start..].copy_from_slice(&code[arg_start..arg_end]);
                Some(u64::from_be_bytes(buf) as usize)
            }
            _ => None,
        }
    };

    // Stack at CODECOPY: [destOffset(0), offset(1), size(2), ...]
    // Reject if size argument isn't exactly 0x20 (32).
    // Larger copies are bulk data loads (role hash tables, etc.) that produce FP.
    let size_sym = state.get_stack(2);
    if sym_to_usize(&size_sym)? != 0x20 {
        return None;
    }

    // Extract the code offset from the `offset` argument (stack position 1).
    let offset = sym_to_usize(&state.get_stack(1))?;

    // Read 32 bytes at the code offset.
    if offset.checked_add(32).is_some_and(|end| end <= code.len()) {
        let mut topic = [0u8; 32];
        topic.copy_from_slice(&code[offset..offset + 32]);
        if is_plausible_event_hash(&topic) {
            return Some(topic);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Sub-class e/f: CrossBlock (BFS backtrack through predecessor blocks)
// ---------------------------------------------------------------------------

fn resolve_cross_block(
    code: &[u8],
    index: &CfgIndex,
    site: &ClassifiedLogSite,
    init_sym_n: usize,
    state_cache: &mut BlockStateCache,
    cont_cache: &mut ContinuationCache,
    out: &mut HashSet<EventSelector>,
) {
    let Some(contexts) = index.contexts_reaching_block.get(&site.site.block_start) else {
        return;
    };

    let init_sym = StackSym::Before(init_sym_n);
    let mut queue: VecDeque<BacktrackState> = VecDeque::new();
    queue.extend(contexts.iter().copied().map(|context| BacktrackState {
        block_start: site.site.block_start,
        context,
        sym: init_sym.clone(),
    }));

    let mut visited: HashSet<BacktrackKey> = HashSet::default();
    let mut processed_states = 0usize;
    let mut pred_steps = 0usize;

    while let Some(state) = queue.pop_front() {
        if processed_states >= MAX_STATES_PER_LOG || pred_steps >= MAX_PRED_STEPS_PER_LOG {
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
                resolve_topic_at_pc(code, pc, index, out);
            }
            StackSym::Before(n) => {
                let continuations =
                    cont_cache.get_or_compute(code, index, state_cache, state.block_start, n);
                for &(pred, ref exit_sym) in continuations {
                    if pred_steps >= MAX_PRED_STEPS_PER_LOG {
                        break;
                    }
                    pred_steps += 1;

                    let context_reaches_pred = index
                        .contexts_reaching_block
                        .get(&pred)
                        .is_some_and(|set| set.contains(&state.context));
                    if !context_reaches_pred {
                        continue;
                    }

                    queue.push_back(BacktrackState {
                        block_start: pred,
                        context: state.context,
                        sym: exit_sym.clone(),
                    });
                }
            }
            StackSym::Pushed(_) | StackSym::Jumpdest(_) => {}
        }
    }
}

/// Unified topic extraction: dispatches to push32/pushn/mload_codecopy based on opcode at `pc`.
fn resolve_topic_at_pc(code: &[u8], pc: usize, index: &CfgIndex, out: &mut HashSet<EventSelector>) {
    let Some(&opcode) = code.get(pc) else {
        return;
    };
    match opcode {
        op::PUSH32 => {
            if let Some(topic) = push32_value(code, pc)
                && is_plausible_event_hash(&topic)
            {
                out.insert(topic);
            }
        }
        op::PUSH5..=op::PUSH31 => {
            if let Some(topic) = pushn_value(code, pc)
                && is_plausible_event_hash(&topic)
            {
                out.insert(topic);
            }
        }
        op::MLOAD => {
            if let Some(block_start) = find_block_start(&index.blocks, pc) {
                resolve_mload_codecopy(code, pc, block_start, out);
            }
        }
        _ => {}
    }
}
