use alloy_primitives::hex;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

fn decode_hex_code(input: &str) -> Result<Vec<u8>, JsError> {
    hex::decode(input).map_err(|e| JsError::new(&format!("Failed to decode code hex input: {e}")))
}

#[wasm_bindgen(typescript_custom_section)]
const DOC_FUNCTION: &'static str = r#"
/**
 * Represents a function found in the contract bytecode
 * @property selector - Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd').
 * @property bytecode_offset - Starting byte offset within the EVM bytecode for the function body.
 * @property arguments - Function argument types in canonical format (e.g., 'uint256,address[]'). Not present if arguments were not extracted
 * @property state_mutability - Function's state mutability ("pure", "view", "payable", or "nonpayable"). Not present if state mutability were not extracted
 */
export type ContractFunction = {
    selector: string,
    bytecode_offset: number,
    arguments?: string,
    state_mutability?: string,
};
"#;
/// @typedef {Object} ContractFunction
/// @property {string} selector - Function selector as a 4-byte hex string without '0x' prefix (e.g., 'aabbccdd')
/// @property {number} bytecode_offset - Starting byte offset within the EVM bytecode for the function body
/// @property {string} [arguments] - Function argument types in canonical format (e.g., 'uint256,address[]'). Not present if arguments were not extracted
/// @property {string} [state_mutability] - Function's state mutability ("pure", "view", "payable", or "nonpayable"). Not present if state mutability were not extracted
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_function() {}
#[derive(Serialize)]
struct JsFunction {
    selector: String,

    #[serde(default, rename = "bytecodeOffset")]
    bytecode_offset: usize,

    arguments: Option<String>,

    #[serde(default, rename = "stateMutability")]
    state_mutability: Option<String>,
}

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
/// @typedef {Object} StorageRecord
/// @property {string} slot - Storage slot number as a hex string (e.g., '0', '1b')
/// @property {number} offset - Byte offset within the storage slot (0-31)
/// @property {string} type - Variable type (e.g., 'uint256', 'mapping(address => uint256)', 'bytes32')
/// @property {string[]} reads - Array of function selectors that read from this storage location
/// @property {string[]} writes - Array of function selectors that write to this storage location
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_storage_record() {}
#[derive(Serialize)]
struct JsStorageRecord {
    slot: String,
    offset: u8,
    r#type: String,
    reads: Vec<String>,
    writes: Vec<String>,
}

#[wasm_bindgen(typescript_custom_section)]
const DOC_CONTRACT: &'static str = r#"
/**
 * Contains the analysis results of a contract
 * @property functions - Array of functions found in the contract. Not present if no functions were extracted.
 * @property storage - Array of storage records found in the contract. Not present if storage layout was not extracted.
 * @see ContractFunction
 * @see StorageRecord
 */
export type Contract = {
    functions?: ContractFunction[],
    storage?: StorageRecord[],
};
"#;
/// @typedef {Object} Contract
/// @property {ContractFunction[]} [functions] - Array of functions found in the contract. Not present if no functions were extracted
/// @property {StorageRecord[]} [storage] - Array of storage records found in the contract. Not present if storage layout was not extracted
#[wasm_bindgen(skip_jsdoc)]
pub fn dummy_contract() {}
#[derive(Serialize)]
struct JsContract {
    functions: Option<Vec<JsFunction>>,
    storage: Option<Vec<JsStorageRecord>>,
}

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
 * @param args.state_mutability - When true, includes state mutability information for functions
 * @param args.storage - When true, includes contract storage layout information
 * @returns Analyzed contract information
 */
export function contractInfo(code: string, {
    selectors?: boolean,
    arguments?: boolean,
    state_mutability?: boolean,
    storage?: boolean
}): Contract;
"#;
/// Analyzes contract bytecode and returns contract information based on specified options.
///
/// @param {string} code - Runtime bytecode as a hex string
/// @param {Object} args - Configuration options for the analysis
/// @param {boolean} [args.selectors] - When true, includes function selectors in the output
/// @param {boolean} [args.arguments] - When true, includes function arguments information
/// @param {boolean} [args.state_mutability] - When true, includes state mutability information for functions
/// @param {boolean} [args.storage] - When true, includes contract storage layout information
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

    let info = crate::contract_info(cargs);

    let functions = info.functions.map(|fns| {
        fns.into_iter()
            .map(|f| JsFunction {
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
            .map(|v| JsStorageRecord {
                slot: hex::encode(v.slot),
                offset: v.offset,
                r#type: v.r#type,
                reads: v.reads.into_iter().map(hex::encode).collect(),
                writes: v.writes.into_iter().map(hex::encode).collect(),
            })
            .collect()
    });

    Ok(serde_wasm_bindgen::to_value(&JsContract {
        functions,
        storage,
    })?)
}
