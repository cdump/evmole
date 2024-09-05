use alloy_primitives::hex::{self, FromHex};
use wasm_bindgen::prelude::*;

fn decode_hex_code(input: &str) -> Result<Vec<u8>, JsError> {
    hex::decode(input).map_err(|e| JsError::new(&format!("Failed to decode code hex input: {e}")))
}

fn decode_hex_selector(input: &str) -> Result<[u8; 4], JsError> {
    <[u8; 4]>::from_hex(input)
        .map_err(|e| JsError::new(&format!("Failed to decode selector hex input: {e}")))
}

/// Extracts function selectors from the given bytecode.
///
/// @param {string} code - Runtime bytecode as a hex string
/// @param {number} gas_limit - Maximum allowed gas usage; set to `0` to use defaults
/// @returns {string[]} Function selectors as a hex strings
#[wasm_bindgen(js_name = functionSelectors, skip_jsdoc)]
pub fn function_selectors(code: &str, gas_limit: u32) -> Result<Vec<String>, JsError> {
    // TODO: accept Uint8Array | str(hex) input
    let c = decode_hex_code(code)?;
    Ok(crate::selectors::function_selectors(&c, gas_limit)
        .into_iter()
        .map(hex::encode)
        .collect())
}

/// Extracts function arguments for a given selector from the bytecode.
///
/// @param {string} code - Runtime bytecode as a hex string
/// @param {string} selector - Function selector as a hex string
/// @param {number} gas_limit - Maximum allowed gas usage; set to `0` to use defaults
/// @returns {string} Function arguments (ex: `uint32,address`)
#[wasm_bindgen(js_name = functionArguments, skip_jsdoc)]
pub fn function_arguments(code: &str, selector: &str, gas_limit: u32) -> Result<String, JsError> {
    let c = decode_hex_code(code)?;
    let s = decode_hex_selector(selector)?;
    Ok(crate::arguments::function_arguments(&c, &s, gas_limit))
}

/// Extracts function state mutability for a given selector from the bytecode.
///
/// @param {string} code - Runtime bytecode as a hex string
/// @param {string} selector - Function selector as a hex string
/// @param {number} gas_limit - Maximum allowed gas usage; set to `0` to use defaults
/// @returns {string} `payable` | `nonpayable` | `view` | `pure`
#[wasm_bindgen(js_name = functionStateMutability, skip_jsdoc)]
pub fn function_state_mutability(
    code: &str,
    selector: &str,
    gas_limit: u32,
) -> Result<String, JsError> {
    let c = decode_hex_code(code)?;
    let s = decode_hex_selector(selector)?;
    Ok(crate::state_mutability::function_state_mutability(&c, &s, gas_limit).as_json_str().to_string())
}
