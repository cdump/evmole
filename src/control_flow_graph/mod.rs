//! Control flow graph analysis for EVM bytecode
//!
//! This module provides types and structures for representing control flow graphs of EVM bytecode.
//! It is primarily used internally by the analyzer and should not be used directly.
//!
//! To analyze EVM bytecode and get its control flow graph, use the [`super::contract_info`]
//! function instead. This module is exported only to make its types and structures available
//! for use with the public API.

use std::collections::BTreeMap;

use initial::initial_blocks;
pub(crate) use reachable::get_reachable_nodes;
use resolver::resolve_dynamic_jumps;

mod initial;
mod reachable;
mod resolver;
mod state;

/// Constant used to mark invalid jump destinations (jumps not to JUMPDEST).
/// Any jump destination value equal to or greater than this constant should be considered invalid.
/// Value is deliberately chosen to be larger than the maximum possible EVM contract
/// code size, ensuring it cannot occur in valid code.
pub const INVALID_JUMP_START: usize = 30_000;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// A basic block in the control flow graph representing a sequence of instructions
/// with a single entry point and a single exit point
pub struct Block {
    /// Byte offset where the block's first opcode begins
    pub start: usize,
    /// Byte offset where the block's last opcode begins
    pub end: usize,

    #[cfg_attr(feature = "serde", serde(flatten))]
    /// Type of the block indicating how control flow continues after this block
    pub btype: BlockType,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// Represents a dynamic jump destination with the path taken to reach it
pub struct DynamicJump {
    /// Sequence of block offsets representing the path taken to reach this jump
    pub path: Vec<usize>,
    /// The resolved destination of the jump, if known
    pub to: Option<usize>,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "type", content = "data")
)]
/// Enum representing the different types of control flow that can occur at the end of a block
pub enum BlockType {
    /// Block ends with terminating instruction
    Terminate {
        /// Whether the termination was successful (true for STOP/RETURN)
        success: bool,
    },
    /// Block ends with an unconditional jump
    Jump {
        /// Destination of the jump
        to: usize,
    },
    /// Block ends with a conditional jump
    Jumpi {
        /// Destination if condition is true
        true_to: usize,
        /// Destination if condition is false
        false_to: usize,
    },
    /// Block ends with an unconditional jump to a dynamically computed destination
    DynamicJump {
        /// Possible jump destinations and paths to reach them
        to: Vec<DynamicJump>,
    },
    /// Block ends with a conditional jump where the true branch is dynamic
    DynamicJumpi {
        /// Possible jump destinations and paths for the true branch
        true_to: Vec<DynamicJump>,
        /// Static destination for the false branch
        false_to: usize,
    },
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// Control flow graph representing the structure and flow of EVM bytecode
pub struct ControlFlowGraph {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serialize::blocks")
    )]
    /// Map of block start offsets to their corresponding blocks
    pub blocks: BTreeMap<usize, Block>,
}

pub(crate) fn basic_blocks(code: &[u8]) -> BTreeMap<usize, Block> {
    initial_blocks(code)
}

pub(crate) fn control_flow_graph(code: &[u8], mut blocks: BTreeMap<usize, Block>) -> ControlFlowGraph {
    blocks = resolve_dynamic_jumps(code, blocks);

    // Blocks reachable from start (pc=0)
    let reachable = get_reachable_nodes(&blocks, 0, None);
    blocks.retain(|start, _| reachable.contains(start));

    ControlFlowGraph { blocks }
}
