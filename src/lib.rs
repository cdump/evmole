//!  Extracts function selectors and arguments from bytecode, even for unverified contracts.
//!
//! Accuracy and speed comparison with other tools, as well as Python and JavaScript implementations, are available on [GitHub](https://github.com/cdump/evmole/tree/master#benchmark).

pub use arguments::function_arguments;
pub use arguments::function_arguments_alloy;
pub use selectors::function_selectors;
pub use state_mutability::function_state_mutability;

#[doc(hidden)]
pub mod arguments;

#[doc(hidden)]
pub mod state_mutability;

mod evm;
mod utils;

#[doc(hidden)]
pub mod selectors;

pub type Selector = [u8; 4];
pub type StateMutability = alloy_dyn_abi::parser::StateMutability;

#[cfg(feature = "python")]
mod interface_py;

#[cfg(feature = "javascript")]
mod interface_js;
