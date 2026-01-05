//! WASM interface for Go bindings via wazero.
//!
//! This module provides C-ABI compatible exports for use with WASM runtimes
//! that don't use wasm-bindgen (like wazero for Go).

use alloy_primitives::hex;
use std::alloc::{Layout, alloc, dealloc};
use std::slice;

use crate::control_flow_graph::BlockType;

/// Allocate memory in WASM linear memory.
/// Returns a pointer to the allocated memory.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc(layout) }
}

/// Deallocate memory previously allocated with wasm_alloc.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_dealloc(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let layout = Layout::from_size_align(size, 1).unwrap();
    unsafe { dealloc(ptr, layout) }
}

/// Options bitmask constants
const OPT_SELECTORS: u32 = 1;
const OPT_ARGUMENTS: u32 = 2;
const OPT_STATE_MUTABILITY: u32 = 4;
const OPT_STORAGE: u32 = 8;
const OPT_DISASSEMBLE: u32 = 16;
const OPT_BASIC_BLOCKS: u32 = 32;
const OPT_CONTROL_FLOW_GRAPH: u32 = 64;

/// Analyze EVM bytecode and return contract information as JSON.
///
/// # Arguments
/// * `code_ptr` - Pointer to bytecode data
/// * `code_len` - Length of bytecode
/// * `opts` - Bitmask of options (see OPT_* constants)
///
/// # Returns
/// Pointer to result buffer. First 4 bytes are little-endian length,
/// followed by JSON data. Caller must free with wasm_dealloc using length+4.
/// Returns null on error.
#[unsafe(no_mangle)]
pub extern "C" fn contract_info(code_ptr: *const u8, code_len: usize, opts: u32) -> *mut u8 {
    // Safety: caller guarantees valid pointer and length
    let code = unsafe {
        if code_ptr.is_null() || code_len == 0 {
            return std::ptr::null_mut();
        }
        slice::from_raw_parts(code_ptr, code_len)
    };

    // Build ContractInfoArgs based on options bitmask
    let mut args = crate::ContractInfoArgs::new(code);

    if opts & OPT_SELECTORS != 0 {
        args = args.with_selectors();
    }
    if opts & OPT_ARGUMENTS != 0 {
        args = args.with_arguments();
    }
    if opts & OPT_STATE_MUTABILITY != 0 {
        args = args.with_state_mutability();
    }
    if opts & OPT_STORAGE != 0 {
        args = args.with_storage();
    }
    if opts & OPT_DISASSEMBLE != 0 {
        args = args.with_disassemble();
    }
    if opts & OPT_BASIC_BLOCKS != 0 {
        args = args.with_basic_blocks();
    }
    if opts & OPT_CONTROL_FLOW_GRAPH != 0 {
        args = args.with_control_flow_graph();
    }

    let info = crate::contract_info(args);

    // Convert to JSON-serializable structure
    let result = ContractResult::from_contract(info);

    // Serialize to JSON
    let json = match serde_json::to_vec(&result) {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };

    // Allocate buffer for length prefix + JSON data
    let total_len = 4 + json.len();
    let ptr = wasm_alloc(total_len);
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    // Write length prefix (little-endian u32)
    unsafe {
        let len_bytes = (json.len() as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_bytes.as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(json.as_ptr(), ptr.add(4), json.len());
    }

    ptr
}

// JSON-serializable result types

#[derive(serde::Serialize)]
struct ContractResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Vec<FunctionResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<Vec<StorageRecordResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disassembled: Option<Vec<(usize, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    basic_blocks: Option<Vec<(usize, usize)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    control_flow_graph: Option<ControlFlowGraphResult>,
}

#[derive(serde::Serialize)]
struct FunctionResult {
    selector: String,
    bytecode_offset: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_mutability: Option<String>,
}

#[derive(serde::Serialize)]
struct StorageRecordResult {
    slot: String,
    offset: u8,
    r#type: String,
    reads: Vec<String>,
    writes: Vec<String>,
}

#[derive(serde::Serialize)]
struct ControlFlowGraphResult {
    blocks: Vec<BlockResult>,
}

#[derive(serde::Serialize)]
struct BlockResult {
    start: usize,
    end: usize,
    #[serde(flatten)]
    btype: BlockTypeResult,
}

#[derive(serde::Serialize)]
#[serde(tag = "type", content = "data")]
enum BlockTypeResult {
    Terminate {
        success: bool,
    },
    Jump {
        to: usize,
    },
    Jumpi {
        true_to: usize,
        false_to: usize,
    },
    DynamicJump {
        to: Vec<DynamicJumpResult>,
    },
    DynamicJumpi {
        true_to: Vec<DynamicJumpResult>,
        false_to: usize,
    },
}

#[derive(serde::Serialize)]
struct DynamicJumpResult {
    path: Vec<usize>,
    to: Option<usize>,
}

impl ContractResult {
    fn from_contract(info: crate::Contract) -> Self {
        let functions = info.functions.map(|fns| {
            fns.into_iter()
                .map(|f| FunctionResult {
                    selector: hex::encode(f.selector),
                    bytecode_offset: f.bytecode_offset,
                    arguments: f.arguments.map(|fargs| {
                        fargs
                            .into_iter()
                            .map(|t| t.sol_type_name().to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    }),
                    state_mutability: f.state_mutability.map(|sm| sm.as_json_str().to_string()),
                })
                .collect()
        });

        let storage = info.storage.map(|st| {
            st.into_iter()
                .map(|v| StorageRecordResult {
                    slot: hex::encode(v.slot),
                    offset: v.offset,
                    r#type: v.r#type,
                    reads: v.reads.into_iter().map(hex::encode).collect(),
                    writes: v.writes.into_iter().map(hex::encode).collect(),
                })
                .collect()
        });

        let control_flow_graph = info.control_flow_graph.map(|cfg| ControlFlowGraphResult {
            blocks: cfg
                .blocks
                .into_values()
                .map(|bl| BlockResult {
                    start: bl.start,
                    end: bl.end,
                    btype: match bl.btype {
                        BlockType::Terminate { success } => BlockTypeResult::Terminate { success },
                        BlockType::Jump { to } => BlockTypeResult::Jump { to },
                        BlockType::Jumpi { true_to, false_to } => {
                            BlockTypeResult::Jumpi { true_to, false_to }
                        }
                        BlockType::DynamicJump { to } => BlockTypeResult::DynamicJump {
                            to: to
                                .into_iter()
                                .map(|v| DynamicJumpResult {
                                    path: v.path,
                                    to: v.to,
                                })
                                .collect(),
                        },
                        BlockType::DynamicJumpi { true_to, false_to } => {
                            BlockTypeResult::DynamicJumpi {
                                true_to: true_to
                                    .into_iter()
                                    .map(|v| DynamicJumpResult {
                                        path: v.path,
                                        to: v.to,
                                    })
                                    .collect(),
                                false_to,
                            }
                        }
                    },
                })
                .collect(),
        });

        ContractResult {
            functions,
            storage,
            disassembled: info.disassembled,
            basic_blocks: info.basic_blocks,
            control_flow_graph,
        }
    }
}
