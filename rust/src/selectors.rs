use crate::evm::{
    op,
    vm::{StepResult, Vm},
    Element, VAL_0_B, VAL_1_B,
};
use crate::Selector;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Label {
    CallData,
    Signature,
}

fn analyze(
    vm: &mut Vm<Label>,
    selectors: &mut Vec<Selector>,
    ret: StepResult<Label>,
    gas_used: &mut u32,
    gas_limit: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    match ret {
          StepResult{op: op::XOR|op::EQ, fa: Some(Element{label: Some(Label::Signature), ..}), sa: Some(s1), ..}
        | StepResult{op: op::XOR|op::EQ, sa: Some(Element{label: Some(Label::Signature), ..}), fa: Some(s1), ..} =>
        {
            selectors.push(s1.data[28..32].try_into().unwrap());
            let v = vm.stack.peek_mut()?;
            v.data = if ret.op == op::XOR { VAL_1_B } else { VAL_0_B };
        }

          StepResult{op: op::SUB, fa: Some(Element{label: Some(Label::Signature), ..}), sa: Some(s1), ..}
        | StepResult{op: op::SUB, sa: Some(Element{label: Some(Label::Signature), ..}), fa: Some(s1), ..} =>
        {
            selectors.push(s1.data[28..32].try_into().unwrap());
        }

          StepResult{op: op::LT|op::GT, fa: Some(Element{label: Some(Label::Signature), ..}), ..}
        | StepResult{op: op::LT|op::GT, sa: Some(Element{label: Some(Label::Signature), ..}), ..} =>
        {
            *gas_used += process(vm.clone(), selectors, (gas_limit - *gas_used) / 2);
            let v = vm.stack.peek_mut()?;
            v.data = if v.data == VAL_0_B { VAL_1_B } else { VAL_0_B };
        }

          StepResult{op: op::SHR, sa: Some(Element{label: Some(Label::CallData), ..}), ..}
        | StepResult{op: op::AND, fa: Some(Element{label: Some(Label::Signature), ..}), ..}
        | StepResult{op: op::AND, sa: Some(Element{label: Some(Label::Signature), ..}), ..}
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
            selectors.push([0; 4]);
        }

        StepResult{op: op::MLOAD, ul: Some(used), ..} =>
        {
            if used.contains(&Label::CallData) {
                let v = vm.stack.peek_mut()?;
                v.label = Some(
                    if v.data[28..32] == vm.calldata.data[0..4] {
                        Label::Signature
                    } else {
                        Label::CallData
                    }
                )
            }
        }

        _ => {}
    }
    Ok(())
}

fn process(mut vm: Vm<Label>, selectors: &mut Vec<Selector>, gas_limit: u32) -> u32 {
    let mut gas_used = 0;
    while !vm.stopped {
        // println!("{:?}\n", vm);
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
/// use hex::decode;
///
/// let code = decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();
///
/// let selectors: Vec<_> = function_selectors(&code, 0);
///
/// assert_eq!(selectors, vec![[0x21, 0x25, 0xb6, 0x5b], [0xb6, 0x9e, 0xf8, 0xa8]])
/// ```
pub fn function_selectors(code: &[u8], gas_limit: u32) -> Vec<Selector> {
    let vm = Vm::<Label>::new(
        code,
        Element {
            data: [0xaa, 0xbb, 0xcc, 0xdd, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            label: Some(Label::CallData),
        },
    );
    let mut selectors: Vec<Selector> = Vec::new();
    process(
        vm,
        &mut selectors,
        if gas_limit == 0 {
            5e5 as u32
        } else {
            gas_limit
        },
    );
    selectors
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
