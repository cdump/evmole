use crate::evm::{
    op,
    vm::{StepResult, Vm},
    Element, U256, VAL_0_B, VAL_1_B,
};
use crate::Selector;
use alloy_primitives::uint;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Label {
    CallData,
    Signature,
    MulSig,
}

const VAL_FFFFFFFF_B: [u8; 32] = uint!(0xffffffff_U256).to_be_bytes();

fn analyze(
    vm: &mut Vm<Label>,
    selectors: &mut BTreeSet<Selector>,
    ret: StepResult<Label>,
    gas_used: &mut u32,
    gas_limit: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    match ret {
          StepResult{op: op::XOR|op::EQ|op::SUB, fa: Some(Element{label: Some(Label::Signature), ..}), sa: Some(s1), ..}
        | StepResult{op: op::XOR|op::EQ|op::SUB, sa: Some(Element{label: Some(Label::Signature), ..}), fa: Some(s1), ..} =>
        {
            selectors.insert(s1.data[28..32].try_into().expect("4 bytes slice is always convertable to Selector"));
            let v = vm.stack.peek_mut()?;
            v.data = if ret.op == op::EQ { VAL_0_B } else { VAL_1_B };
        }

          StepResult{op: op::LT|op::GT, fa: Some(Element{label: Some(Label::Signature), ..}), ..}
        | StepResult{op: op::LT|op::GT, sa: Some(Element{label: Some(Label::Signature), ..}), ..} =>
        {
            *gas_used += process(vm.clone(), selectors, (gas_limit - *gas_used) / 2);
            let v = vm.stack.peek_mut()?;
            v.data = if v.data == VAL_0_B { VAL_1_B } else { VAL_0_B };
        }

          StepResult{op: op::MUL, fa: Some(Element{label: Some(Label::Signature), ..}), ..}
        | StepResult{op: op::MUL, sa: Some(Element{label: Some(Label::Signature), ..}), ..}
        | StepResult{op: op::SHR, sa: Some(Element{label: Some(Label::MulSig), ..}), ..} =>
        {
            vm.stack.peek_mut()?.label = Some(Label::MulSig);
        }

        // Vyper _selector_section_dense()/_selector_section_sparse()
        // (sig MOD n_buckets) or (sig AND (n_buckets-1))
          StepResult{op: op @ op::MOD, fa: Some(Element{label: Some(Label::MulSig | Label::Signature), ..}), sa: Some(ot), ..}
        | StepResult{op: op @ op::AND, fa: Some(Element{label: Some(Label::Signature), ..}), sa: Some(ot), ..}
        | StepResult{op: op @ op::AND, sa: Some(Element{label: Some(Label::Signature), ..}), fa: Some(ot), ..} =>
        {
            if op == op::AND && ot.data == VAL_FFFFFFFF_B {
                vm.stack.peek_mut()?.label = Some(Label::Signature);
            } else {
                let ot8: Result<u8, _> = U256::from_be_bytes(ot.data).try_into();
                if let Ok(ma) = ot8 {
                    let to = if op == op::MOD { ma } else { ma + 1 };
                    for m in 1..to {
                        let mut vm_clone = vm.clone();
                        vm_clone.stack.peek_mut()?.data = U256::from(m).to_be_bytes();
                        *gas_used += process(vm_clone, selectors, (gas_limit - *gas_used) / (ma as u32));
                        if *gas_used > gas_limit {
                            break;
                        }
                    }
                    vm.stack.peek_mut()?.data = VAL_0_B;
                }
            }
        }

          StepResult{op: op::SHR, sa: Some(Element{label: Some(Label::CallData), ..}), ..}
        | StepResult{op: op::DIV, fa: Some(Element{label: Some(Label::CallData), ..}), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            if v.data[28..32] == vm.calldata.data[0..4] {
                v.label = Some(Label::Signature);
            }
        }

          StepResult{op: op::AND, fa: Some(Element{label: Some(Label::CallData), ..}), ..}
        | StepResult{op: op::AND, sa: Some(Element{label: Some(Label::CallData), ..}), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            v.label = Some(Label::CallData);
        }

        StepResult{op: op::ISZERO, fa: Some(Element{label: Some(Label::Signature), ..}), ..} =>
        {
            selectors.insert([0; 4]);
        }

        StepResult{op: op::MLOAD, ul: Some(used), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            if used.contains(&Label::CallData) && v.data[28..32] == vm.calldata.data[0..4] {
                v.label = Some(Label::Signature);
            }
        }

        _ => {}
    }
    Ok(())
}

fn process(mut vm: Vm<Label>, selectors: &mut BTreeSet<Selector>, gas_limit: u32) -> u32 {
    let mut gas_used = 0;
    while !vm.stopped {
        if cfg!(feature = "trace") {
            println!(
                "selectors: {:?}",
                selectors
                    .iter()
                    .map(|s| format!("{:02x}{:02x}{:02x}{:02x},", s[0], s[1], s[2], s[3]))
                    .collect::<Vec<String>>()
            );
            println!("{:?}\n", vm);
        }
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // eprintln!("{}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > gas_limit {
            break;
        }

        if analyze(&mut vm, selectors, ret, &mut gas_used, gas_limit).is_err() {
            break;
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
///
/// # Examples
///
/// ```
/// use evmole::function_selectors;
/// use alloy_primitives::hex;
///
/// let code = hex::decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();
///
/// let selectors: Vec<_> = function_selectors(&code, 0);
///
/// assert_eq!(selectors, vec![[0x21, 0x25, 0xb6, 0x5b], [0xb6, 0x9e, 0xf8, 0xa8]])
/// ```
pub fn function_selectors(code: &[u8], gas_limit: u32) -> Vec<Selector> {
    let vm = Vm::<Label>::new(
        code,
        Element {
            data: [
                0xaa, 0xbb, 0xcc, 0xdd, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
            ],
            label: Some(Label::CallData),
        },
    );
    let mut selectors = BTreeSet::new();
    process(
        vm,
        &mut selectors,
        if gas_limit == 0 {
            5e5 as u32
        } else {
            gas_limit
        },
    );
    selectors.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_code() {
        let s = function_selectors(&[], 0);
        assert_eq!(s.len(), 0);
    }
}
