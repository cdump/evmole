//! Analyzes EVM bytecode to extract contract information, even for unverified contracts.
//!
//! Use the [`contract_info()`] function with its builder pattern to analyze contracts.
//!
//! Accuracy and speed comparison with other tools, as well as Python and JavaScript libraries,
//! are available on [GitHub](https://github.com/cdump/evmole/tree/master#benchmark)

pub use contract_info::contract_info;
pub use contract_info::{Contract, ContractInfoArgs, Function};
pub use storage::StorageRecord;

mod arguments;
mod contract_info;
mod evm;
mod selectors;
mod state_mutability;
mod storage;
mod utils;
mod collections;
pub mod control_flow_graph;

#[cfg(feature = "serde")]
mod serialize;

/// A 4-byte function selector
pub type Selector = [u8; 4];

/// A 32-byte storage slot identifier in EVM storage.
pub type Slot = [u8; 32];

/// Function's state mutability
pub type StateMutability = alloy_dyn_abi::parser::StateMutability;

/// A dynamic Solidity type
pub type DynSolType = alloy_dyn_abi::DynSolType;

#[cfg(feature = "python")]
mod interface_py;

#[cfg(feature = "javascript")]
mod interface_js;

#[cfg(feature = "c_api")]
mod interface_c;
