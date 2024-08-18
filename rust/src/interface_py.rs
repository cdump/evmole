use std::borrow::Cow;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

fn input_to_bytes<'a>(code: &'a Bound<'a, PyAny>) -> PyResult<Cow<'a, [u8]>> {
    if let Ok(s) = code.downcast::<PyString>() {
        let str_slice = s
            .to_str()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let v = hex::decode(strip_hex_prefix(str_slice))
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

fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

/// function_selectors(code: str | bytes, gas_limit: int = 5e5)
/// --
///
/// Extracts function selectors
#[pyfunction]
#[pyo3(signature = (code, gas_limit=5e5 as u32))]
fn function_selectors(code: &Bound<'_, PyAny>, gas_limit: u32) -> PyResult<Vec<String>> {
    let code_bytes = input_to_bytes(code)?;
    Ok(crate::selectors::function_selectors(&code_bytes, gas_limit)
        .into_iter()
        .map(hex::encode)
        .collect())
}

/// function_arguments(code: str | bytes, selector: str | bytes, gas_limit: int = 5e5)
/// --
///
/// Extracts function arguments
#[pyfunction]
#[pyo3(signature = (code, selector, gas_limit=5e5 as u32))]
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
        <[u8; 4]>::try_from(selectors_ref).unwrap()
    };

    Ok(crate::arguments::function_arguments(
        &code_bytes,
        &sel,
        gas_limit,
    ))
}

#[pymodule]
fn evmole(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(function_selectors, m)?)?;
    m.add_function(wrap_pyfunction!(function_arguments, m)?)?;
    Ok(())
}
