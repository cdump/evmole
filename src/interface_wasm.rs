#[no_mangle]
pub extern "C" fn function_selectors(
    code_ptr: *const u8,
    code_len: usize,
    gas_limit: u32,
    result_len_ptr: *mut u32,
    result_ptr: *mut u8,
    result_capacity: usize,
) -> u32 {
    let code = unsafe { std::slice::from_raw_parts(code_ptr, code_len) };
    let selectors = crate::selectors::function_selectors(code, gas_limit);

    let flattened: Vec<u8> = selectors.into_iter().flatten().collect();
    let result_len = flattened.len();

    unsafe {
        *result_len_ptr = result_len as u32;
    }

    if result_len > result_capacity {
        // Buffer too small
        return 1;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(flattened.as_ptr(), result_ptr, result_len);
    }
    // Success
    0
}
