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

#[pyclass(name = "Function")]
#[derive(Clone)]
struct PyFunction {
    #[pyo3(get)]
    selector: String,

    #[pyo3(get)]
    bytecode_offset: usize,

    #[pyo3(get)]
    arguments: Option<String>,

    #[pyo3(get)]
    state_mutability: Option<String>,
}

impl PyFunction {
    fn repr(&self) -> String {
        format!(
            "Function(selector={},bytecode_offset={},arguments={},state_mutability={})",
            self.selector,
            self.bytecode_offset,
            self.arguments.as_deref().unwrap_or("None"),
            self.state_mutability.as_deref().unwrap_or("None"),
        )
    }
}

#[pymethods]
impl PyFunction {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Ok(PyFunction::repr(&slf.borrow()))
    }
}

#[pyclass(name = "StorageRecord")]
#[derive(Clone)]
struct PyStorageRecord {
    slot: String,
    offset: u8,
    r#type: String,
    reads: Vec<String>,
    writes: Vec<String>,
}

impl PyStorageRecord {
    fn repr(&self) -> String {
        format!(
            "StorageRecord(slot={},offset={},type={},reads={:?},writes={:?})",
            self.slot, self.offset, self.r#type, self.reads, self.writes
        )
    }
}

#[pymethods]
impl PyStorageRecord {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Ok(PyStorageRecord::repr(&slf.borrow()))
    }
}

#[pyclass(name = "Contract")]
struct PyContract {
    #[pyo3(get)]
    functions: Option<Vec<PyFunction>>,

    #[pyo3(get)]
    storage: Option<Vec<PyStorageRecord>>,
}

#[pymethods]
impl PyContract {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        Ok(format!(
            "Contract(functions={},storage={})",
            if let Some(ref v) = slf.borrow().functions {
                format!(
                    "[{}]",
                    v.iter().map(|v| v.repr()).collect::<Vec<_>>().join(",")
                )
            } else {
                "None".to_string()
            },
            if let Some(ref v) = slf.borrow().storage {
                format!(
                    "[{}]",
                    v.iter().map(|v| v.repr()).collect::<Vec<_>>().join(",")
                )
            } else {
                "None".to_string()
            },
        ))
    }
}

#[pyfunction]
#[pyo3(signature = (code, *, selectors=false, arguments=false, state_mutability=false, storage=false))]
fn contract_info(
    code: &Bound<'_, PyAny>,
    selectors: bool,
    arguments: bool,
    state_mutability: bool,
    storage: bool,
) -> PyResult<PyContract> {
    let code_bytes = input_to_bytes(code)?;
    let mut args = crate::ContractInfoArgs::new(&code_bytes);

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

    let info = crate::contract_info(args);

    let functions = info.functions.map(|fns| {
        fns.into_iter()
            .map(|f| PyFunction {
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
            .map(|v| PyStorageRecord {
                slot: hex::encode(v.slot),
                offset: v.offset,
                r#type: v.r#type,
                reads: v.reads.into_iter().map(hex::encode).collect(),
                writes: v.writes.into_iter().map(hex::encode).collect(),
            })
            .collect()
    });

    Ok(PyContract { functions, storage })
}

#[pymodule]
fn evmole(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(contract_info, m)?)?;
    Ok(())
}
