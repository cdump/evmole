use crate::{
    evm::{
        element::Element,
        op,
        vm::{StepResult, Vm},
        U256, VAL_0_B,
    },
    utils::{elabel, execute_until_function_start},
    Selector, StateMutability,
};

mod calldata;
use calldata::CallDataImpl;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Label {
    CallValue,
    IsZero,
}

const fn create_opcode_lookup_table<const N: usize>(ops: [op::OpCode; N]) -> [bool; 256] {
    let mut res = [false; 256];
    let mut i = 0;
    while i < N {
        res[ops[i] as usize] = true;
        i += 1;
    }
    res
}

const OP_NOT_VIEW: [bool; 256] = create_opcode_lookup_table([
    op::CALL,
    op::CALLCODE,
    op::CREATE,
    op::CREATE2,
    op::DELEGATECALL,
    op::SELFDESTRUCT,
    op::SSTORE,
]);

const OP_NOT_PURE: [bool; 256] = create_opcode_lookup_table([
    op::BALANCE,
    op::BASEFEE,
    op::BLOBBASEFEE,
    op::BLOBHASH,
    op::BLOCKHASH,
    op::CALLER,
    op::CHAINID,
    op::COINBASE,
    op::EXTCODECOPY,
    op::EXTCODEHASH,
    op::EXTCODESIZE,
    op::GASLIMIT,
    op::GASPRICE,
    op::NUMBER,
    op::ORIGIN,
    op::PREVRANDAO,
    op::SELFBALANCE,
    op::SLOAD,
    op::STATICCALL,
    op::TIMESTAMP,
]);

fn analyze_payable(
    mut vm: Vm<Label, CallDataImpl>,
    gas_limit: u32,
    call_value: u32,
) -> (bool, u32) {
    let mut gas_used = 0;
    let mut last_jumpi_callvalue = false;

    while !vm.stopped {
        if cfg!(feature = "trace_mutability") {
            println!("{vm:?}\n");
        }
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // println!("{}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > gas_limit {
            break;
        }

        match ret {
            StepResult {
                op: op::CALLVALUE, ..
            } => {
                if let Ok(s) = vm.stack.peek_mut() {
                    s.data = U256::from(call_value).to_be_bytes();
                    s.label = Some(Label::CallValue);
                } else {
                    break;
                }
            }

            StepResult {
                op: op::ISZERO,
                args: [elabel!(Label::CallValue), ..],
                ..
            } => {
                vm.stack
                    .peek_mut()
                    .expect("results is always pushed in vm.rs")
                    .label = Some(Label::IsZero);
            }

            StepResult {
                op: op::JUMPI,
                args: [_, sa, ..],
                ..
            } => {
                last_jumpi_callvalue =
                    sa.label == Some(Label::IsZero) || sa.label == Some(Label::CallValue);
            }

            StepResult {
                op: op::REVERT,
                args: [_, sa, ..],
                ..
            } => {
                if last_jumpi_callvalue && sa.data == VAL_0_B {
                    return (false, gas_used);
                }
            }

            _ => (),
        }
    }
    (true, gas_used)
}

struct ViewPureResult {
    pub view: bool,
    pub pure: bool,
}

fn analyze_view_pure_internal(
    mut vm: Vm<Label, CallDataImpl>,
    vpr: &mut ViewPureResult,
    gas_limit: u32,
    depth: u32,
) -> u32 {
    let mut gas_used = 0;

    if depth == 0 {
        if let Some(g) = execute_until_function_start(&mut vm, gas_limit) {
            gas_used += g;
        } else {
            return gas_used;
        }
    }

    while !vm.stopped && vpr.view {
        if cfg!(feature = "trace_mutability") {
            println!("{vm:?}\n");
        }
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // println!("{}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > gas_limit {
            break;
        }

        match ret.op {
            op::JUMPI => {
                let other_pc = usize::try_from(&ret.args[0]).expect("set to usize in vm.rs");

                if depth < 8 && gas_used < gas_limit {
                    let mut cloned = vm.fork();
                    cloned.pc = other_pc;
                    gas_used += analyze_view_pure_internal(
                        cloned,
                        vpr,
                        (gas_limit - gas_used) / 2,
                        depth + 1,
                    );
                } else {
                    // println!("depth overflow");
                }
            }

            _ => {
                if OP_NOT_VIEW[ret.op as usize] {
                    vpr.view = false;
                    vpr.pure = false;
                } else if OP_NOT_PURE[ret.op as usize] {
                    vpr.pure = false;
                }
            }
        };
    }
    gas_used
}

fn analyze_view_pure(vm: Vm<Label, CallDataImpl>, gas_limit: u32) -> ViewPureResult {
    let mut ret = ViewPureResult {
        view: true,
        pure: true,
    };
    analyze_view_pure_internal(vm, &mut ret, gas_limit, 0);
    ret
}

/// Extracts function state mutability
///
/// # Arguments
///
/// * `code` - A slice of deployed contract bytecode
/// * `selector` - A function selector
/// * `gas_limit` - Maximum allowed gas usage; set to `0` to use defaults
/// ```
pub fn function_state_mutability(
    code: &[u8],
    selector: &Selector,
    gas_limit: u32,
) -> StateMutability {
    let calldata = CallDataImpl {
        selector: *selector,
    };
    let vm = Vm::new(code, &calldata);

    let real_gas_limit = if gas_limit == 0 {
        5e5 as u32
    } else {
        gas_limit
    };

    let (is_payable, gas_used) = analyze_payable(vm.fork(), real_gas_limit / 2, 1);
    if is_payable {
        StateMutability::Payable
    } else {
        let gas_remaining = real_gas_limit - gas_used.min(real_gas_limit / 2);
        let vpr = analyze_view_pure(vm, gas_remaining);
        if vpr.pure {
            StateMutability::Pure
        } else if vpr.view {
            StateMutability::View
        } else {
            StateMutability::NonPayable
        }
    }
}
