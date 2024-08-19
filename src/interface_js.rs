use hex::FromHex;
use wasm_bindgen::prelude::*;

fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

fn decode_hex_code(input: &str) -> Result<Vec<u8>, JsError> {
    hex::decode(strip_hex_prefix(input))
        .map_err(|e| JsError::new(&format!("Failed to decode code hex input: {}", e)))
}

fn decode_hex_selector(input: &str) -> Result<[u8; 4], JsError> {
    <[u8; 4]>::from_hex(strip_hex_prefix(input))
        .map_err(|e| JsError::new(&format!("Failed to decode selector hex input: {}", e)))
}

#[wasm_bindgen(js_name = functionSelectors)]
pub fn function_selectors(code: &str, gas_limit: u32) -> Result<Vec<String>, JsError> {
    // TODO: accept Uint8Array | str(hex) input
    let c = decode_hex_code(code)?;
    Ok(crate::selectors::function_selectors(&c, gas_limit)
        .into_iter()
        .map(hex::encode)
        .collect())
}

#[wasm_bindgen(js_name = functionArguments)]
pub fn function_arguments(code: &str, selector: &str, gas_limit: u32) -> Result<String, JsError> {
    let c = decode_hex_code(code)?;
    let s = decode_hex_selector(selector)?;
    Ok(crate::arguments::function_arguments(&c, &s, gas_limit))
}
