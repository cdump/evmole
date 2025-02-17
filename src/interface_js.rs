use alloy_primitives::hex;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

fn decode_hex_code(input: &str) -> Result<Vec<u8>, JsError> {
    hex::decode(input).map_err(|e| JsError::new(&format!("Failed to decode code hex input: {e}")))
}

// {{{ Contract
#[wasm_bindgen(typescript_custom_section)]
const DOC_CONTRACT: &'static str = r#"
/**
 * Contains the analysis results of a contract
 * @property functions - Array of functions found in the contract. Not present if no functions were extracted.
 * @property storage - Array of storage records found in the contract. Not present if storage layout was not extracted.
 * @property disassembled - Array of bytecode instructions, where each element is a tuple of [offset: number, instruction: string]
 * @property basicBlocks - Array of basic blocks found in the contract. Not present if basic blocks were not analyzed.
 * @property controlFlowGraph - Control flow graph representation. Not present if CFG was not generated.
 * @see ContractFunction
 * @see StorageRecord
 */
export type Contract = {
    functions?: ContractFunction[],
    storage?: StorageRecord[],
    disassembled?: [number, string][],
    basicBlocks?: [number, number][],
    controlFlowGraph?: ControlFlowGraph,
};
"#;
/// @typedef {Object} Contract
/// @description Contains the analysis results of a contract
/// @property {ContractFunction[]} [functions] - Array of functions found in the contract. Not present if no functions were extracted
/// @property {StorageRecord[]} [storage] - Array of storage records found in the contract. Not present if storage layout was not extracted
/// @property {Array<Array<number|string>>} [disassembled] - Array of bytecode instructions, where each element is [offset, instruction]
/// @property {Array<Array<number>>} [basicBlocks] - Array of basic blocks found in the contract. Not present if basic blocks were not analyzed.
/// @property {ControlFlowGraph} [controlFlowGraph] - Control flow graph representation. Not present if CFG was not generated.
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_contract() {}
// }}}

// {{{ Function
#[wasm_bindgen(typescript_custom_section)]
const DOC_FUNCTION: &'static str = r#"
/**
 * Represents a function found in the contract bytecode
 * @property selector - Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd').
 * @property bytecodeOffset - Starting byte offset within the EVM bytecode for the function body.
 * @property arguments - Function argument types in canonical format (e.g., 'uint256,address[]'). Not present if arguments were not extracted
 * @property stateMutability - Function's state mutability ("pure", "view", "payable", or "nonpayable"). Not present if state mutability were not extracted
 */
export type ContractFunction = {
    selector: string,
    bytecodeOffset: number,
    arguments?: string,
    stateMutability?: string,
};
"#;
/// @typedef {Object} ContractFunction
/// @description Represents a function found in the contract bytecode
/// @property {string} selector - Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd')
/// @property {number} bytecodeOffset - Starting byte offset within the EVM bytecode for the function body
/// @property {string} [arguments] - Function argument types in canonical format (e.g., 'uint256,address[]'). Not present if arguments were not extracted
/// @property {string} [stateMutability] - Function's state mutability ("pure", "view", "payable", or "nonpayable"). Not present if state mutability were not extracted
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_function() {}
// }}}

// {{{ StorageRecord
#[wasm_bindgen(typescript_custom_section)]
const DOC_STORAGE: &'static str = r#"
/**
 * Represents a storage record found in the contract
 * @property slot - Storage slot number as a hex string (e.g., '0', '1b').
 * @property offset - Byte offset within the storage slot (0-31).
 * @property type - Variable type (e.g., 'uint256', 'mapping(address => uint256)', 'bytes32').
 * @property reads - Array of function selectors that read from this storage location.
 * @property writes - Array of function selectors that write to this storage location.
 */
export type StorageRecord = {
    slot: string,
    offset: number,
    type: string,
    reads: string[],
    writes: string[]
};
"#;
/// Represents a storage record found in the contract
///
/// @typedef {Object} StorageRecord
/// @property {string} slot - Storage slot number as a hex string (e.g., '0', '1b')
/// @property {number} offset - Byte offset within the storage slot (0-31)
/// @property {string} type - Variable type (e.g., 'uint256', 'mapping(address => uint256)', 'bytes32')
/// @property {string[]} reads - Array of function selectors that read from this storage location
/// @property {string[]} writes - Array of function selectors that write to this storage location
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_storage_record() {}
// }}}

// {{{ ControlFlowGraph
#[wasm_bindgen(typescript_custom_section)]
const DOC_CONTROL_FLOW_GRAPH: &'static str = r#"
/**
   Represents the control flow graph of the contract bytecode
 * @property blocks - List of basic blocks in the control flow graph
 */
export type ControlFlowGraph = {
    blocks: Block[],
};
"#;
/// @typedef {Object} ControlFlowGraph
/// @description Represents the control flow graph of the contract bytecode
/// @property {Block[]} blocks - List of basic blocks in the control flow graph
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_control_flow_graph() {}
// }}}

// {{{ Block
#[wasm_bindgen(typescript_custom_section)]
const DOC_BLOCK: &'static str = r#"
/**
   Represents a basic block in the control flow graph
 * @property start - Byte offset where the block's first opcode begins
 * @property end - Byte offset where the block's last opcode begins
 * @property type - Block type
 * @property data - Type-specific data
 */
export type Block = {
    start: number,
    end: number,
    type: 'Terminate' | 'Jump' | 'Jumpi' | 'DynamicJump' | 'DynamicJumpi';
} & (
    | {
          type: 'Terminate';
          data: { success: boolean };
      }
    | {
          type: 'Jump';
          data: { to: number };
      }
    | {
          type: 'Jumpi';
          data: { true_to: number; false_to: number };
      }
    | {
          type: 'DynamicJump';
          data: { to: DynamicJump };
      }
    | {
          type: 'DynamicJumpi';
          data: { true_to: DynamicJump, false_to: number};
      }
);
"#;

/// @typedef {Object} Block
/// @description Represents a basic block in the control flow graph
/// @property {number} start - Byte offset where the block's first opcode begins
/// @property {number} end - Byte offset where the block's last opcode begins
/// @property {('Terminate'|'Jump'|'Jumpi'|'DynamicJump'|'DynamicJumpi')} type - Block type
/// @property {(DataTerminate|DataJump|DataJumpi|DataDynamicJump|DataDynamicJumpi)} data - Type Type-specific block data
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_block() {}
// }}}

// {{{ Data Terminate Block
/// @typedef {Object} DataTerminate
/// @property {boolean} success - true for normal termination (STOP/RETURN), false for REVERT/INVALID
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_block_data_terminate() {}
// }}}

// {{{ Data Jump Block
/// @typedef {Object} DataJump
/// @property {number} to - Destination basic block offset
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_block_data_jump() {}
// }}}

// {{{ Data Jumpi Block
/// @typedef {Object} DataJumpi
/// @property {number} true_to - Destination if condition is true
/// @property {number} false_to - Destination if condition is false (fall-through)
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_block_data_jumpi() {}
// }}}

// {{{ Data DynamicJump Block
/// @typedef {Object} DataDynamicJump
/// @property {DynamicJump} to - Possible computed jump destinations
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_block_data_dynamic_jump() {}
// }}}

// {{{ Data DynamicJumpi Block
/// @typedef {Object} DataDynamicJumpi
/// @property {DynamicJump} true_to - Possible computed jump destinations if true
/// @property {number} false_to - Destination if condition is false (fall-through)
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_block_data_dynamic_jumpi() {}
// }}}

// {{{ DynamicJump
#[wasm_bindgen(typescript_custom_section)]
const DOC_DYNAMIC_JUMP: &'static str = r#"
/**
   Represents a dynamic jump destination in the control flow
 * @property path - Path of basic blocks leading to this jump
 * @property to - Target basic block offset if known
 */
export type DynamicJump = {
    path: number[];
    to?: number;
};
"#;
/// @typedef {Object} DynamicJump
/// @description Represents a dynamic jump destination in the control flow
/// @property {number[]} path - Path of basic blocks leading to this jump
/// @property {number} [to] - Target basic block offset if known. Optional
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_dynamic_jump() {}
// }}}

// {{{ ContractInfo function
#[derive(Deserialize)]
struct ContractInfoArgs {
    #[serde(default)]
    selectors: bool,

    #[serde(default)]
    arguments: bool,

    #[serde(default, rename = "stateMutability")]
    state_mutability: bool,

    #[serde(default)]
    storage: bool,

    #[serde(default)]
    disassemble: bool,

    #[serde(default, rename = "basicBlocks")]
    basic_blocks: bool,

    #[serde(default, rename = "controlFlowGraph")]
    control_flow_graph: bool,
}

#[wasm_bindgen(typescript_custom_section)]
const DOC_CONTRACT_INFO: &'static str = r#"
/**
 * Analyzes contract bytecode and returns contract information based on specified options.
 *
 * @param code - Runtime bytecode as a hex string
 * @param args - Configuration options for the analysis
 * @param args.selectors - When true, includes function selectors in the output
 * @param args.arguments - When true, includes function arguments information
 * @param args.stateMutability - When true, includes state mutability information for functions
 * @param args.storage - When true, includes contract storage layout information
 * @param args.disassemble - When true, includes disassembled bytecode
 * @param args.basicBlocks - When true, includes basic block analysis
 * @param args.controlFlowGraph - When true, includes control flow graph analysis
 * @returns Analyzed contract information
 */
export function contractInfo(code: string, args: {
    selectors?: boolean,
    arguments?: boolean,
    stateMutability?: boolean,
    storage?: boolean,
    disassemble?: boolean,
    basicBlocks?: boolean,
    controlFlowGraph?: boolean
}): Contract;
"#;
/// Analyzes contract bytecode and returns contract information based on specified options.
///
/// @param {string} code - Runtime bytecode as a hex string
/// @param {Object} args - Configuration options for the analysis
/// @param {boolean} [args.selectors] - When true, includes function selectors in the output
/// @param {boolean} [args.arguments] - When true, includes function arguments information
/// @param {boolean} [args.stateMutability] - When true, includes state mutability information for functions
/// @param {boolean} [args.storage] - When true, includes contract storage layout information
/// @param {boolean} [args.disassemble] - When true, includes disassembled bytecode
/// @param {boolean} [args.basicBlocks] - When true, includes basic block analysis
/// @param {boolean} [args.controlFlowGraph] - When true, includes control flow graph analysis
/// @returns {Contract} Analyzed contract information
#[wasm_bindgen(js_name = contractInfo, skip_typescript, skip_jsdoc)]
pub fn contract_info(code: &str, args: JsValue) -> Result<JsValue, JsError> {
    let c = decode_hex_code(code)?;
    let args: ContractInfoArgs = serde_wasm_bindgen::from_value(args)?;

    let mut cargs = crate::ContractInfoArgs::new(&c);

    if args.selectors {
        cargs = cargs.with_selectors();
    }
    if args.arguments {
        cargs = cargs.with_arguments();
    }
    if args.state_mutability {
        cargs = cargs.with_state_mutability();
    }
    if args.storage {
        cargs = cargs.with_storage();
    }
    if args.disassemble {
        cargs = cargs.with_disassemble();
    }
    if args.basic_blocks {
        cargs = cargs.with_basic_blocks();
    }
    if args.control_flow_graph {
        cargs = cargs.with_control_flow_graph();
    }

    let info = crate::contract_info(cargs);
    Ok(serde_wasm_bindgen::to_value(&info)?)
}
// }}}
