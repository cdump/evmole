//!  Extracts function selectors and arguments from bytecode, even for unverified contracts.
//!
//! Accuracy and speed comparison with other tools, as well as Python and JavaScript implementations, are available on [GitHub](https://github.com/cdump/evmole/tree/master#benchmark).

pub use arguments::function_arguments;
pub use arguments::function_arguments_typed;
pub use selectors::function_selectors;

#[doc(hidden)]
pub mod arguments;

mod evm;

#[doc(hidden)]
pub mod selectors;

pub type Selector = [u8; 4];

#[cfg(feature = "python")]
mod interface_py;

#[cfg(feature = "javascript")]
mod interface_js;
