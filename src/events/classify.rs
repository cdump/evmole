use std::collections::BTreeMap;

use crate::collections::{HashMap, HashSet};
use crate::control_flow_graph::{
    Block, BlockType, INVALID_JUMP_START, basic_blocks, control_flow_graph,
    state::{StackSym, State},
};
use crate::evm::{code_iterator::iterate_code, op};
use crate::selectors::function_selectors;

#[derive(Clone, Copy, Debug)]
pub(super) struct LogSite {
    pub pc: usize,
    pub block_start: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LogSiteClass {
    /// Sub-class a: topic0 produced by PUSH32 in the same block.
    Push32 { topic_pc: usize },
    /// Sub-class b: topic0 produced by PUSH5..PUSH31 in the same block.
    PushN { topic_pc: usize },
    /// Sub-class c: topic0 produced by MLOAD (preceded by CODECOPY pattern).
    MloadCodecopy { mload_pc: usize },
    /// Sub-class e/f: topic0 comes from a predecessor block (Before(n)).
    CrossBlock { init_sym_n: usize },
}

#[derive(Clone, Debug)]
pub(super) struct ClassifiedLogSite {
    pub site: LogSite,
    pub class: LogSiteClass,
}

pub(super) struct CfgIndex {
    pub blocks: BTreeMap<usize, Block>,
    pub preds_by_block: HashMap<usize, HashSet<usize>>,
    pub contexts_reaching_block: HashMap<usize, HashSet<usize>>,
}

pub(super) fn classify_log_sites(code: &[u8]) -> (CfgIndex, Vec<ClassifiedLogSite>) {
    let index = build_cfg_index(code);
    if index.blocks.is_empty() {
        return (index, Vec::new());
    }

    let mut out = Vec::new();
    for site in collect_log_sites(code, &index.blocks) {
        if let Some(class) = classify_one(code, &index.blocks, site) {
            out.push(ClassifiedLogSite { site, class });
        }
    }
    out.sort_unstable_by(|a, b| a.site.pc.cmp(&b.site.pc));
    (index, out)
}

fn classify_one(
    code: &[u8],
    blocks: &BTreeMap<usize, Block>,
    site: LogSite,
) -> Option<LogSiteClass> {
    let sym = topic0_symbol_at_log(code, blocks, site)?;
    match sym {
        StackSym::Other(pc) => {
            let &opcode = code.get(pc)?;
            match opcode {
                op::PUSH32 => Some(LogSiteClass::Push32 { topic_pc: pc }),
                op::PUSH5..=op::PUSH31 => Some(LogSiteClass::PushN { topic_pc: pc }),
                op::MLOAD => Some(LogSiteClass::MloadCodecopy { mload_pc: pc }),
                _ => None,
            }
        }
        StackSym::Before(n) => Some(LogSiteClass::CrossBlock { init_sym_n: n }),
        StackSym::Pushed(_) | StackSym::Jumpdest(_) => None,
    }
}

// ---------------------------------------------------------------------------
// CFG construction
// ---------------------------------------------------------------------------

// TODO: control_flow_graph() prunes blocks unreachable from PC=0 via resolved edges.
// Internal functions called through unresolved dynamic jumps (e.g. Solidity internal
// _transfer/_mint) are lost. This causes ~33 FN on largest1k where PUSH32+LOG exist
// in pruned blocks. Fix requires improving dynamic jump resolution in
// control_flow_graph/resolver.rs so these blocks enter the reachable set.
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
    set.insert(0);
    set.extend(selectors.into_values());
    let mut out: Vec<usize> = set.into_iter().collect();
    out.sort_unstable();
    out
}

// ---------------------------------------------------------------------------
// LOG site collection & symbolic helpers
// ---------------------------------------------------------------------------

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

fn topic0_symbol_at_log(
    code: &[u8],
    blocks: &BTreeMap<usize, Block>,
    log_site: LogSite,
) -> Option<StackSym> {
    let block = blocks.get(&log_site.block_start)?;
    let mut state = State::new();
    if let Some(prev_pc) = find_prev_instruction_pc(code, block.start, log_site.pc) {
        let _ = state.exec(code, block.start, Some(prev_pc));
    }
    // LOG1..LOG4: stack is [offset, size, topic0, ...]; topic0 is at position 2
    Some(state.get_stack(2))
}

pub(super) fn find_prev_instruction_pc(
    code: &[u8],
    start_pc: usize,
    target_pc: usize,
) -> Option<usize> {
    let mut prev = None;
    for (pc, _) in iterate_code(code, start_pc, Some(target_pc)) {
        if pc == target_pc {
            return prev;
        }
        prev = Some(pc);
    }
    None
}

pub(super) fn find_block_start(blocks: &BTreeMap<usize, Block>, pc: usize) -> Option<usize> {
    let (start, block) = blocks.range(..=pc).next_back()?;
    if pc <= block.end { Some(*start) } else { None }
}
