use crate::Selector;
use crate::{
    evm::{
        U256, VAL_0_B, VAL_1_B,
        calldata::CallData,
        element::Element,
        op,
        vm::{StepResult, Vm},
    },
    utils::{elabel, match_first_two},
};
use alloy_primitives::{hex, uint};
use std::collections::BTreeMap;

mod calldata;
use calldata::CallDataImpl;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Label {
    CallData,
    Signature,
    MulSig,
    SelCmp(Selector),
}

const VAL_FFFFFFFF_B: [u8; 32] = uint!(0xffffffff_U256).to_be_bytes();

fn analyze(
    vm: &mut Vm<Label, CallDataImpl>,
    selectors: &mut BTreeMap<Selector, usize>,
    ret: StepResult<Label>,
) -> Result<u8, Box<dyn std::error::Error>> {
    match ret {
        StepResult {
            op: op::XOR | op::EQ | op::SUB,
            args: match_first_two!(elabel!(Label::Signature), s1),
            ..
        } => {
            let selector: Selector = s1.data[28..32]
                .try_into()
                .expect("4 bytes slice is always convertible to Selector");
            *vm.stack.peek_mut()? = Element {
                data: if ret.op == op::EQ { VAL_0_B } else { VAL_1_B },
                label: Some(Label::SelCmp(selector)),
            };

            // Vyper _selector_section_dense()/_selector_section_sparse()
            if ret.op == op::EQ && vm.stack.data.len() >= 2 {
                let fh = vm.stack.data[vm.stack.data.len() - 2].data;
                let target = u16::from_be_bytes([fh[29], fh[30]]) as usize;
                // assert!(vm.code[target] == op::JUMPDEST);
                if target < vm.code.len() && vm.code[target] == op::JUMPDEST {
                    selectors.insert(selector, target);
                }
            }
        }

        StepResult {
            op: op::JUMPI,
            args: [fa, elabel!(Label::SelCmp(selector)), ..],
            ..
        } => {
            let pc = usize::try_from(fa).expect("set to usize in vm.rs");
            selectors.insert(selector, pc);
        }

        StepResult {
            op: op::LT | op::GT,
            args: match_first_two!(elabel!(Label::Signature), _),
            ..
        } => {
            vm.stack.peek_mut()?.data = VAL_0_B;
            return Ok(2);
        }

        StepResult {
            op: op::MUL,
            args: match_first_two!(elabel!(Label::Signature), _),
            ..
        }
        | StepResult {
            op: op::SHR,
            args: [_, elabel!(Label::MulSig), ..],
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::MulSig);
        }

        // Vyper _selector_section_dense()/_selector_section_sparse()
        // (sig MOD n_buckets) or (sig AND (n_buckets-1))
        StepResult {
            op: op @ op::MOD,
            args: [elabel!(Label::MulSig | Label::Signature), ot, ..],
            ..
        }
        | StepResult {
            op: op @ op::AND,
            args: match_first_two!(elabel!(Label::Signature), ot),
            ..
        } => {
            if op == op::AND && ot.data == VAL_FFFFFFFF_B {
                vm.stack.peek_mut()?.label = Some(Label::Signature);
            } else if let Ok(ma) = u8::try_from(ot) {
                if ma < u8::MAX {
                    let to = if op == op::MOD { ma } else { ma + 1 };
                    vm.stack.peek_mut()?.data = VAL_0_B;
                    return Ok(to);
                }
            }
        }

        StepResult {
            op: op::SHR,
            args: [_, elabel!(Label::CallData), ..],
            ..
        }
        | StepResult {
            op: op::DIV,
            args: [elabel!(Label::CallData), ..],
            ..
        } => {
            let v = vm.stack.peek_mut()?;
            if v.data[28..32] == vm.calldata.selector() {
                v.label = Some(Label::Signature);
            }
        }

        StepResult {
            op: op::AND,
            args: match_first_two!(elabel!(Label::CallData), _),
            ..
        } => {
            let v = vm.stack.peek_mut()?;
            v.label = Some(Label::CallData);
        }

        StepResult {
            op: op::ISZERO,
            args: [elabel!(Label::SelCmp(selector)), ..],
            ..
        } => {
            let v = vm.stack.peek_mut()?;
            v.label = Some(Label::SelCmp(selector));
        }

        StepResult {
            op: op::ISZERO,
            args: [elabel!(Label::Signature), ..],
            ..
        } => {
            let v = vm.stack.peek_mut()?;
            v.label = Some(Label::SelCmp([0; 4]));
        }

        StepResult {
            op: op::MLOAD,
            memory_load: Some(memory_load),
            ..
        } => {
            let v = vm.stack.peek_mut()?;
            if memory_load.chunks.iter().any(|e| e.src_label == Label::CallData) {
                v.label = Some(if v.data[28..32] == vm.calldata.selector() {
                    Label::Signature
                } else {
                    Label::CallData
                });
            }
        }

        StepResult { op: op::GAS, .. } => {
            vm.stopped = true;
        }

        _ => {}
    }
    Ok(0)
}

fn process(
    mut vm: Vm<Label, CallDataImpl>,
    selectors: &mut BTreeMap<Selector, usize>,
    gas_limit: u32,
) -> u32 {
    let mut gas_used = 0;
    while !vm.stopped {
        if cfg!(feature = "trace_selectors") {
            println!(
                "selectors: {:?}",
                selectors
                    .iter()
                    .map(|(s, p)| (hex::encode(s), *p))
                    .collect::<Vec<(String, usize)>>()
            );
            println!("{:?}\n", vm);
        }
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // eprintln!("vm error: {:?}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > gas_limit {
            // eprintln!("gas overflow");
            break;
        }

        match analyze(&mut vm, selectors, ret) {
            Ok(0) => {}
            Ok(to) => {
                for m in 1..to {
                    let mut vm_clone = vm.fork();
                    vm_clone.stack.peek_mut().expect("already unwraped").data =
                        U256::from(m).to_be_bytes();
                    let gas = process(vm_clone, selectors, (gas_limit - gas_used) / (to as u32));
                    gas_used += gas;
                    if gas_used > gas_limit {
                        // eprintln!("gas overflow");
                        return gas_used;
                    }
                }
            }
            Err(_e) => {
                // eprintln!("analyze error: {:?}", _e);
                return gas_used;
            }
        }
    }
    gas_used
}

/// Extracts function selectors
///
/// # Arguments
///
/// * `code` - A slice of deployed contract bytecode
/// * `gas_limit` - Maximum allowed gas usage; set to `0` to use defaults
/// ```
pub fn function_selectors(code: &[u8], gas_limit: u32) -> (BTreeMap<Selector, usize>, u32) {
    let vm = Vm::new(code, &CallDataImpl {});
    let mut selectors = BTreeMap::new();
    let gas_used = process(
        vm,
        &mut selectors,
        if gas_limit == 0 {
            5e5 as u32
        } else {
            gas_limit
        },
    );
    (selectors, gas_used)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_code() {
        let (s, _) = function_selectors(&[], 0);
        assert_eq!(s.len(), 0);
    }
}
