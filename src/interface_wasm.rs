//! WASM interface for EVMole contract analysis

use crate::ContractInfoArgs;

/// Extracts contract information from EVM bytecode and returns JSON-encoded result.
///
/// # Options Flags (bitfield at `options_ptr`)
///
/// * Bit 0 (0x01): Function selectors
/// * Bit 1 (0x02): Function arguments
/// * Bit 2 (0x04): State mutability
/// * Bit 3 (0x08): Storage layout
/// * Bit 4 (0x10): Disassemble bytecode
/// * Bit 5 (0x20): Basic blocks
///
/// # Returns
///
/// * `0` - Success
/// * `1` - Buffer too small (check `result_len_ptr` for required size)
/// * `2` - Serialization error
#[unsafe(no_mangle)]
pub extern "C" fn contract_info(
    code_ptr: *const u8,
    code_len: usize,
    options_ptr: *const u8,
    result_len_ptr: *mut u32,
    result_ptr: *mut u8,
    result_capacity: usize,
) -> u32 {
    let code = unsafe { std::slice::from_raw_parts(code_ptr, code_len) };
    let options = unsafe { *options_ptr };

    // Parse option flags (bitfield)
    let selectors = (options & (1 << 0)) != 0; // Bit 0: selectors
    let arguments = (options & (1 << 1)) != 0; // Bit 1: arguments
    let state_mutability = (options & (1 << 2)) != 0; // Bit 2: state mutability
    let storage = (options & (1 << 3)) != 0; // Bit 3: storage layout
    let disassemble = (options & (1 << 4)) != 0; // Bit 4: disassembly
    let basic_blocks = (options & (1 << 5)) != 0; // Bit 5: basic blocks

    // Build contract info args
    let mut args = ContractInfoArgs::new(code);
    if selectors {
        args = args.with_selectors();
    }
    if arguments {
        args = args.with_arguments();
    }
    if state_mutability {
        args = args.with_state_mutability();
    }
    if storage {
        args = args.with_storage();
    }
    if disassemble {
        args = args.with_disassemble();
    }
    if basic_blocks {
        args = args.with_basic_blocks();
    }

    // Get contract info
    let contract = crate::contract_info(args);

    // Serialize to JSON
    let json = match serde_json::to_vec(&contract) {
        Ok(j) => j,
        Err(_) => return 2, // Serialization error
    };

    let result_len = json.len();

    unsafe {
        *result_len_ptr = result_len as u32;
    }

    if result_len > result_capacity {
        // Buffer too small - caller should allocate larger buffer and retry
        return 1;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(json.as_ptr(), result_ptr, result_len);
    }

    0 // Success
}
