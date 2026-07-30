use crate::{CborMetadata, Selector, SelectorDispatch};
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
    CallDataSize,
    Signature,
    MulSig,
    SelCmp(Selector),
}

const VAL_FFFFFFFF_B: [u8; 32] = uint!(0xffffffff_U256).to_be_bytes();

fn vyper_dense_target(code: &[u8], func_info: &[u8; 32], selector: Selector) -> Option<usize> {
    const METHOD_ID_BYTES: usize = 4;
    const FUNCTION_LABEL_BYTES: usize = 2;

    // Vyper right-aligns `method_id | function_label | metadata` for MLOAD.
    // The contract-wide metadata width depends on its largest minimum calldata size.
    for metadata_bytes in 1..=3 {
        let selector_start =
            func_info.len() - metadata_bytes - FUNCTION_LABEL_BYTES - METHOD_ID_BYTES;
        let label_start = selector_start + METHOD_ID_BYTES;

        if func_info[selector_start..label_start] != selector {
            continue;
        }

        let target = u16::from_be_bytes([func_info[label_start], func_info[label_start + 1]]);
        let target = target as usize;
        if target < code.len() && code[target] == op::JUMPDEST {
            return Some(target);
        }
    }

    None
}

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

            // Vyper _selector_section_dense()
            if ret.op == op::EQ && vm.stack.data.len() >= 2 {
                let fh = vm.stack.data[vm.stack.data.len() - 2].data;
                if let Some(target) = vyper_dense_target(vm.code, &fh, selector) {
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
            op: op::LT | op::GT,
            args: match_first_two!(elabel!(Label::CallDataSize), _),
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::CallDataSize);
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
            } else if let Ok(ma) = u8::try_from(ot)
                && ma < u8::MAX
            {
                let to = if op == op::MOD { ma } else { ma + 1 };
                vm.stack.peek_mut()?.data = VAL_0_B;
                return Ok(to);
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

        // Vyper O(1) selector dispatchers pack the selector check together with
        // a calldata-size guard, notably for method IDs with trailing zero bytes.
        StepResult {
            op: op::AND,
            args:
                match_first_two!(
                    elabel!(Label::SelCmp(selector)),
                    elabel!(Label::CallDataSize)
                ),
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::SelCmp(selector));
        }

        StepResult {
            op: op::CALLDATASIZE,
            ..
        } => {
            vm.stack.peek_mut()?.label = Some(Label::CallDataSize);
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
            if memory_load
                .chunks
                .iter()
                .any(|e| e.src_label == Label::CallData)
            {
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
            println!("{vm:?}\n");
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

fn function_selectors_with_calldata_len(
    code: &[u8],
    gas_limit: u32,
    calldata_len: usize,
) -> (BTreeMap<Selector, usize>, u32) {
    let calldata = CallDataImpl::new(calldata_len);
    let vm = Vm::new(code, &calldata);
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

pub(crate) fn function_selectors(
    code: &[u8],
    gas_limit: u32,
    metadata: Option<&CborMetadata>,
) -> (BTreeMap<Selector, (usize, SelectorDispatch)>, u32) {
    let (all, mut gas_used) = function_selectors_with_calldata_len(code, gas_limit, 4);
    if all.is_empty() {
        return (BTreeMap::new(), gas_used);
    }

    let (mut short, short_gas_used) = function_selectors_with_calldata_len(code, gas_limit, 3);
    gas_used = gas_used.saturating_add(short_gas_used);
    short.retain(|selector, _| all.contains_key(selector));

    let has_four_byte_only = all.keys().any(|selector| !short.contains_key(selector));
    let modern_solc = crate::metadata::solc_version(metadata)
        .is_some_and(|version| version[0] > 0 || version[1] >= 5);

    let classified = all
        .into_iter()
        .map(|(selector, bytecode_offset)| {
            let dispatch = if short.contains_key(&selector) && (has_four_byte_only || modern_solc) {
                SelectorDispatch::Fallback
            } else {
                SelectorDispatch::Abi
            };
            (selector, (bytecode_offset, dispatch))
        })
        .collect();
    (classified, gas_used)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_code() {
        let (s, _) = function_selectors(&[], 0, None);
        assert_eq!(s.len(), 0);
    }
}
