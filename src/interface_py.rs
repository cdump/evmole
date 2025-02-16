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

#[pymodule]
mod evmole {
    use super::*;

    #[pyclass(name = "Function", get_all)]
    #[derive(Clone)]
    struct PyFunction {
        selector: String,
        bytecode_offset: usize,
        arguments: Option<String>,
        state_mutability: Option<String>,
    }

    #[pymethods]
    impl PyFunction {
        fn __repr__(&self) -> String {
            format!(
                "Function(selector={:?}, bytecode_offset={}, arguments={}, state_mutability={})",
                self.selector,
                self.bytecode_offset,
                self.arguments
                    .as_deref()
                    .map_or_else(|| "None".to_string(), |v| format!("\"{v}\"")),
                self.state_mutability
                    .as_deref()
                    .map_or_else(|| "None".to_string(), |v| format!("\"{v}\"")),
            )
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

    #[pymethods]
    impl PyStorageRecord {
        fn __repr__(&self) -> String {
            format!(
                "StorageRecord(slot=\"{}\", offset={}, type=\"{}\", reads={:?}, writes={:?})",
                self.slot, self.offset, self.r#type, self.reads, self.writes
            )
        }
    }

    #[pyclass(name = "Contract", get_all)]
    struct PyContract {
        functions: Option<Vec<PyFunction>>,
        storage: Option<Vec<PyStorageRecord>>,
        disassembled: Option<Vec<(usize, String)>>,
    }

    #[pymethods]
    impl PyContract {
        fn __repr__(&self) -> String {
            format!(
                "Contract(functions={}, storage={}, disassembled={})",
                self.functions.as_ref().map_or_else(||  "None".to_string(), |v|
                    format!(
                        "[{}]",
                        v.iter().map(|v| v.__repr__()).collect::<Vec<_>>().join(", ")
                    )
                ),
                self.storage.as_ref().map_or_else(|| "None".to_string(), |v|
                    format!(
                        "[{}]",
                        v.iter().map(|v| v.__repr__()).collect::<Vec<_>>().join(", ")
                    )
                ),
                self.disassembled.as_ref().map_or_else(|| "None".to_string(), |v|
                    format!("{:?}", v)
                ),
            )
        }
    }

    #[pyfunction]
    #[pyo3(signature = (code, *, selectors=false, arguments=false, state_mutability=false, storage=false, disassemble=false))]
    #[allow(clippy::too_many_arguments)]
    fn contract_info(
        code: &Bound<'_, PyAny>,
        selectors: bool,
        arguments: bool,
        state_mutability: bool,
        storage: bool,
        disassemble: bool,
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
        if disassemble {
            args = args.with_disassemble();
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
                            .join(", ")
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

        Ok(PyContract {
            functions,
            storage,
            disassembled: info.disassembled,
        })
    }
}
