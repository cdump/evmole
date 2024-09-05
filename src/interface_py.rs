use alloy_primitives::hex;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use std::borrow::Cow;

fn input_to_bytes<'a>(code: &'a Bound<'a, PyAny>) -> PyResult<Cow<'a, [u8]>> {
    if let Ok(s) = code.downcast::<PyString>() {
        let str_slice = s
            .to_str()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let v = hex::decode(str_slice)
            .map_err(|e| PyValueError::new_err(format!("failed to parse hex: {}", e)))?;
        Ok(Cow::Owned(v))
    } else if let Ok(b) = code.downcast::<PyBytes>() {
        Ok(Cow::Borrowed(b.as_bytes()))
    } else {
        Err(PyTypeError::new_err(
            "input should be 'str' (hex) or 'bytes'",
        ))
    }
}

/// Extracts function selectors from the given bytecode.
///
/// Args:
///     code (Union[bytes, str]): Runtime bytecode as a hex string or bytes.
///     gas_limit (int, optional): Maximum gas to use. Defaults to 500000.
///
/// Returns:
///     List[str]: List of selectors encoded as hex strings.
#[pyfunction]
#[pyo3(signature = (code, gas_limit=500_000))]
fn function_selectors(code: &Bound<'_, PyAny>, gas_limit: u32) -> PyResult<Vec<String>> {
    let code_bytes = input_to_bytes(code)?;
    Ok(crate::selectors::function_selectors(&code_bytes, gas_limit)
        .into_iter()
        .map(hex::encode)
        .collect())
}

/// Extracts function arguments for a given selector from the bytecode.
///
/// Args:
///     code (Union[bytes, str]): Runtime bytecode as a hex string or bytes.
///     selector (Union[bytes, str]): Function selector as a hex string or bytes.
///     gas_limit (int, optional): Maximum gas to use. Defaults to 50000.
///
/// Returns:
///     str: Arguments of the function.
#[pyfunction]
#[pyo3(signature = (code, selector, gas_limit=50_000))]
fn function_arguments(
    code: &Bound<'_, PyAny>,
    selector: &Bound<'_, PyAny>,
    gas_limit: u32,
) -> PyResult<String> {
    let code_bytes = input_to_bytes(code)?;
    let selector_bytes = input_to_bytes(selector)?;
    let selectors_ref = selector_bytes.as_ref();
    let sel = if selectors_ref.len() != 4 {
        return Err(PyValueError::new_err("selector should be 4 bytes length"));
    } else {
        <[u8; 4]>::try_from(selectors_ref).expect("len checked above")
    };

    Ok(crate::arguments::function_arguments(
        &code_bytes,
        &sel,
        gas_limit,
    ))
}

/// Extracts function state mutability for a given selector from the bytecode.
///
/// Args:
///     code (Union[bytes, str]): Runtime bytecode as a hex string or bytes.
///     selector (Union[bytes, str]): Function selector as a hex string or bytes.
///     gas_limit (int, optional): Maximum gas to use. Defaults to 100000.
///
/// Returns:
///     str: "payable" | "nonpayable" | "view" | "pure"
#[pyfunction]
#[pyo3(signature = (code, selector, gas_limit=500_000))]
fn function_state_mutability(
    code: &Bound<'_, PyAny>,
    selector: &Bound<'_, PyAny>,
    gas_limit: u32,
) -> PyResult<String> {
    let code_bytes = input_to_bytes(code)?;
    let selector_bytes = input_to_bytes(selector)?;
    let selectors_ref = selector_bytes.as_ref();
    let sel = if selectors_ref.len() != 4 {
        return Err(PyValueError::new_err("selector should be 4 bytes length"));
    } else {
        <[u8; 4]>::try_from(selectors_ref).expect("len checked above")
    };

    Ok(
        crate::state_mutability::function_state_mutability(&code_bytes, &sel, gas_limit)
            .as_json_str()
            .to_string(),
    )
}

#[pymodule]
fn evmole(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(function_selectors, m)?)?;
    m.add_function(wrap_pyfunction!(function_arguments, m)?)?;
    m.add_function(wrap_pyfunction!(function_state_mutability, m)?)?;
    Ok(())
}
