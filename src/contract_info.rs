use crate::{
    arguments::function_arguments,
    control_flow_graph::basic_blocks,
    control_flow_graph::{control_flow_graph, ControlFlowGraph},
    evm::code_iterator::disassemble,
    selectors::function_selectors,
    state_mutability::function_state_mutability,
    storage::contract_storage,
};
use crate::{DynSolType, Selector, StateMutability, StorageRecord};

/// Represents a public smart contract function
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Function {
    /// Function selector (4 bytes)
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serialize::selector")
    )]
    pub selector: Selector,

    /// The starting byte offset within the EVM bytecode for the function body
    #[cfg_attr(feature = "serde", serde(rename = "bytecodeOffset"))]
    pub bytecode_offset: usize,

    /// Function arguments
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serialize::arguments")
    )]
    pub arguments: Option<Vec<DynSolType>>,

    /// State mutability
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serialize::state_mutability",
            rename = "stateMutability"
        )
    )]
    pub state_mutability: Option<StateMutability>,
}

/// Contains analyzed information about a smart contract
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Contract {
    /// List of contract functions with their metadata
    pub functions: Option<Vec<Function>>,

    /// Contract storage layout
    pub storage: Option<Vec<StorageRecord>>,

    /// Disassembled code
    pub disassembled: Option<Vec<(usize, String)>>,

    /// Basic blocks representing sequences of instructions that execute sequentially
    #[cfg_attr(feature = "serde", serde(rename = "basicBlocks"))]
    pub basic_blocks: Option<Vec<(usize, usize)>>,

    /// Control flow graph representing the program's execution paths
    #[cfg_attr(feature = "serde", serde(rename = "controlFlowGraph"))]
    pub control_flow_graph: Option<ControlFlowGraph>,
}

/// Builder for configuring contract analysis parameters
///
/// See [`contract_info`] for usage examples.
#[derive(Default)]
pub struct ContractInfoArgs<'a> {
    code: &'a [u8],

    need_selectors: bool,
    need_arguments: bool,
    need_state_mutability: bool,
    need_storage: bool,
    need_disassemble: bool,
    need_basic_blocks: bool,
    need_control_flow_graph: bool,
}

impl<'a> ContractInfoArgs<'a> {
    /// Creates a new instance of contract analysis configuration
    ///
    /// # Arguments
    ///
    /// * `code` - A slice of deployed contract bytecode
    pub fn new(code: &'a [u8]) -> Self {
        ContractInfoArgs {
            code,
            ..Default::default()
        }
    }

    /// Enables the extraction of function selectors
    pub fn with_selectors(mut self) -> Self {
        self.need_selectors = true;
        self
    }

    /// Enables the extraction of function arguments
    pub fn with_arguments(mut self) -> Self {
        self.need_selectors = true;
        self.need_arguments = true;
        self
    }

    /// Enables the extraction of state mutability
    pub fn with_state_mutability(mut self) -> Self {
        self.need_selectors = true;
        self.need_state_mutability = true;
        self
    }

    /// Enables the extraction of the contract's storage layout
    pub fn with_storage(mut self) -> Self {
        self.need_selectors = true;
        self.need_arguments = true;
        self.need_storage = true;
        self
    }

    /// Enables disassemble bytecodes into individual opcodes
    pub fn with_disassemble(mut self) -> Self {
        self.need_disassemble = true;
        self
    }

    /// Enables the extraction of basic blocks from the bytecode
    pub fn with_basic_blocks(mut self) -> Self {
        self.need_basic_blocks = true;
        self
    }

    /// Enables the generation of a control flow graph (CFG)
    pub fn with_control_flow_graph(mut self) -> Self {
        self.need_basic_blocks = true;
        self.need_control_flow_graph = true;
        self
    }
}

/// Extracts information about a smart contract from its EVM bytecode.
///
/// # Parameters
///
/// - `args`: A [`ContractInfoArgs`] instance specifying what data to extract from the provided
///   bytecode. Use the builder-style methods on `ContractInfoArgs` (e.g., `.with_selectors()`,
///   `.with_arguments()`) to enable specific analyses.
///
/// # Returns
///
/// Returns a [`Contract`] object containing the requested smart contract information. The
/// `Contract` struct wraps optional fields depending on the configuration provided in `args`.
/// # Examples
///
/// ```
/// use evmole::{ContractInfoArgs, StateMutability, contract_info};
/// use alloy_primitives::hex;
///
/// let code = hex::decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();
///
/// // Extract function selectors and their state mutability
/// let args = ContractInfoArgs::new(&code)
///     .with_selectors()
///     .with_state_mutability();
///
/// let info = contract_info(args);
/// let fns = info.functions.unwrap();
/// assert_eq!(fns.len(), 2);
/// assert_eq!(fns[0].selector, [0x21, 0x25, 0xb6, 0x5b]);
/// assert_eq!(fns[0].state_mutability, Some(StateMutability::Pure));
/// ```
pub fn contract_info(args: ContractInfoArgs) -> Contract {
    const GAS_LIMIT: u32 = 0;

    let functions = if args.need_selectors {
        let (selectors, _selectors_gas_used) = function_selectors(args.code, GAS_LIMIT);
        Some(
            selectors
                .into_iter()
                .map(|(selector, bytecode_offset)| Function {
                    selector,
                    arguments: if args.need_arguments {
                        Some(function_arguments(args.code, &selector, GAS_LIMIT))
                    } else {
                        None
                    },
                    state_mutability: if args.need_state_mutability {
                        Some(function_state_mutability(args.code, &selector, GAS_LIMIT))
                    } else {
                        None
                    },
                    bytecode_offset,
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    //TODO: filter fns by state_mutability if available
    let storage = if args.need_storage {
        let fns = functions
            .as_ref()
            .expect("enabled on with_storage()")
            .iter()
            .map(|f| (f.selector, f.bytecode_offset, f.arguments.as_ref().unwrap()));
        Some(contract_storage(args.code, fns, GAS_LIMIT))
    } else {
        None
    };

    let disassembled = if args.need_disassemble {
        Some(disassemble(args.code))
    } else {
        None
    };

    let (basic_blocks, control_flow_graph): (Option<Vec<_>>, _) = if args.need_basic_blocks {
        let bb = basic_blocks(args.code);
        let blocks = Some(bb.values().map(|bl| (bl.start, bl.end)).collect());
        let cfg = if args.need_control_flow_graph {
            Some(control_flow_graph(args.code, bb))
        } else {
            None
        };
        (blocks, cfg)
    } else {
        (None, None)
    };

    Contract {
        functions,
        storage,
        disassembled,
        basic_blocks,
        control_flow_graph,
    }
}
