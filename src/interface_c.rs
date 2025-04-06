use alloy_primitives::hex;
use std::ffi::{c_char, c_int, CStr, CString};

/// Converts a C string to a Rust string.
///
/// # Safety
///
/// The pointer must point to a valid C-style null-terminated string.
unsafe fn c_str_to_string(ptr: *const c_char) -> Result<String, &'static str> {
    if ptr.is_null() {
        return Err("Null pointer provided");
    }

    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| "Invalid UTF-8 in string")
}

/// Decode a hexadecimal string into bytes.
fn decode_hex_code(input: &str) -> Result<Vec<u8>, &'static str> {
    hex::decode(input).map_err(|_| "Failed to decode hex input")
}

/// Free memory allocated by this library.
///
/// # Safety
///
/// The pointer must have been allocated by this library.
#[no_mangle]
pub unsafe extern "C" fn evmole_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

#[repr(C)]
pub struct EvmoleContractInfoOptions {
    /// When true, includes function selectors in the output.
    selectors: c_int,
    /// When true, includes function arguments information.
    arguments: c_int,
    /// When true, includes state mutability information for functions.
    state_mutability: c_int,
    /// When true, includes contract storage layout information.
    storage: c_int,
    /// When true, includes disassembled bytecode.
    disassemble: c_int,
    /// When true, includes basic block analysis.
    basic_blocks: c_int,
    /// When true, includes control flow graph analysis.
    control_flow_graph: c_int,
}

/// Analyzes contract bytecode and returns contract information in JSON format.
///
/// # Parameters
///
/// * `code`: Runtime bytecode as a hex string.
/// * `options`: Configuration options for the analysis.
///
/// # Returns
///
/// * A JSON string containing the analysis results. Memory must be freed using evmole_free().
/// * NULL on error, and sets *error_msg to a string describing the error. Memory must be freed using evmole_free().
///
/// # Safety
///
/// The `code` pointer must point to a valid C-style null-terminated string.
/// The error_msg pointer must be valid if the function returns NULL.
#[no_mangle]
pub unsafe extern "C" fn evmole_contract_info(
    code: *const c_char,
    options: EvmoleContractInfoOptions,
    error_msg: *mut *mut c_char,
) -> *mut c_char {
    // Set error message to NULL by default
    if !error_msg.is_null() {
        *error_msg = std::ptr::null_mut();
    }

    // Convert code to Rust string
    let code_str = match c_str_to_string(code) {
        Ok(s) => s,
        Err(e) => {
            if !error_msg.is_null() {
                *error_msg = CString::new(e).unwrap_or_default().into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    // Decode hex code
    let code_bytes = match decode_hex_code(&code_str) {
        Ok(b) => b,
        Err(e) => {
            if !error_msg.is_null() {
                *error_msg = CString::new(e).unwrap_or_default().into_raw();
            }
            return std::ptr::null_mut();
        }
    };

    // Build arguments
    let mut args = crate::ContractInfoArgs::new(&code_bytes);

    if options.selectors != 0 {
        args = args.with_selectors();
    }
    if options.arguments != 0 {
        args = args.with_arguments();
    }
    if options.state_mutability != 0 {
        args = args.with_state_mutability();
    }
    if options.storage != 0 {
        args = args.with_storage();
    }
    if options.disassemble != 0 {
        args = args.with_disassemble();
    }
    if options.basic_blocks != 0 {
        args = args.with_basic_blocks();
    }
    if options.control_flow_graph != 0 {
        args = args.with_control_flow_graph();
    }

    // Run analysis
    let info = crate::contract_info(args);

    // Serialize to JSON
    match serde_json::to_string(&info) {
        Ok(json) => match CString::new(json) {
            Ok(cstr) => cstr.into_raw(),
            Err(e) => {
                if !error_msg.is_null() {
                    *error_msg = CString::new(format!("Failed to convert to C string: {}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_msg.is_null() {
                *error_msg = CString::new(format!("Failed to serialize to JSON: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            std::ptr::null_mut()
        }
    }
}
