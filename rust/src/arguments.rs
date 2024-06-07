use crate::{
    evm::{
        op,
        vm::{StepResult, Vm},
        Element, U256, VAL_0_B, VAL_1, VAL_1_B, VAL_32_B,
    },
    Selector,
};
use std::collections::{BTreeMap, BTreeSet};

const VAL_2_B: [u8; 32] = ruint::uint!(2_U256).to_be_bytes();
const VAL_4_B: [u8; 32] = ruint::uint!(4_U256).to_be_bytes();
const VAL_5_B: [u8; 32] = ruint::uint!(5_U256).to_be_bytes();
const VAL_131072_B: [u8; 32] = ruint::uint!(131072_U256).to_be_bytes();

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Label {
    CallData,
    Arg(u32, bool),
    ArgDynamicLength(u32),
    ArgDynamic(u32),
    IsZeroResult(u32, bool),
}

struct ArgsResult {
    pub args: BTreeMap<u32, String>,
    pub not_bool: BTreeSet<u32>,
}
impl ArgsResult {
    pub fn new() -> ArgsResult {
        ArgsResult {
            args: BTreeMap::new(),
            not_bool: BTreeSet::new(),
        }
    }

    pub fn set(&mut self, offset: u32, atype: &str) {
        self.args.insert(offset, atype.to_string());
    }

    pub fn set_if(&mut self, offset: u32, if_val: &str, atype: &str) {
        if let Some(v) = self.args.get_mut(&offset) {
            if v == if_val {
                *v = atype.to_string();
            }
        } else if atype.is_empty() {
            self.args.insert(offset, atype.to_string());
        }
    }

    pub fn mark_not_bool(&mut self, offset: u32) {
        self.not_bool.insert(offset);
        self.set_if(offset, "bool", "");
    }

    pub fn join_to_string(&self) -> String {
        let a: Vec<_> = self
            .args
            .values()
            .map(|v| if !v.is_empty() { v } else { "uint256" })
            .collect();

        a.join(",")
    }
}

fn analyze(
    vm: &mut Vm<Label>,
    args: &mut ArgsResult,
    ret: StepResult<Label>,
) -> Result<(), Box<dyn std::error::Error>> {
    match ret {
        StepResult{op: op::CALLDATASIZE, ..} =>
        {
            let v = vm.stack.peek_mut()?;
            v.data = VAL_131072_B;
        }

        StepResult{op: op::CALLDATALOAD, fa: Some(Element{label: Some(Label::Arg(off, _)), ..}), ..} =>
        {
            args.set(off, "bytes");
            let v = vm.stack.peek_mut()?;
            *v = Element {
                data: VAL_1_B,
                label: Some(Label::ArgDynamicLength(off)),
            };
        }

        StepResult{op: op::CALLDATALOAD, fa: Some(Element{label: Some(Label::ArgDynamic(off)), ..}), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            *v = Element {
                data: [0; 32],
                label: Some(Label::Arg(off, true)),
            };
        }

        StepResult{op: op::CALLDATALOAD, fa: Some(el), ..} =>
        {
            let off256: U256 = el.into();
            let offr: Result<u32, _> = off256.try_into();
            if let Ok(off) = offr {
                if (4..131072 - 1024).contains(&off) {
                    /* trustedForwarder */
                    let v = vm.stack.peek_mut()?;
                    *v = Element {
                        data: [0; 32],
                        label: Some(Label::Arg(off, false)),
                    };
                    args.set_if(off, "", "");
                }
            }
        }

          StepResult{op: op::ADD, fa: Some(Element{label: Some(Label::Arg(off, _)), ..}), sa: Some(ot), ..}
        | StepResult{op: op::ADD, sa: Some(Element{label: Some(Label::Arg(off, _)), ..}), fa: Some(ot), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            v.label = Some(if ot.data == VAL_4_B {
                Label::Arg(off, false)
            } else {
                Label::ArgDynamic(off)
            });
            args.mark_not_bool(off);
        },

          StepResult{op: op::ADD, fa: Some(Element{label: Some(Label::ArgDynamic(off)), ..}), ..}
        | StepResult{op: op::ADD, sa: Some(Element{label: Some(Label::ArgDynamic(off)), ..}), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            v.label = Some(Label::ArgDynamic(off));
        }

        StepResult{op: op::SHL, fa: Some(ot), sa: Some(Element{label: Some(Label::ArgDynamicLength(off)), ..}), ..} =>
        {
            if ot.data == VAL_5_B {
                args.set(off, "uint256[]");
            } else if ot.data == VAL_1_B {
                args.set(off, "string");
            }
        }

          StepResult{op: op::MUL, fa: Some(Element{label: Some(Label::ArgDynamicLength(off)), ..}), sa: Some(ot), ..}
        | StepResult{op: op::MUL, sa: Some(Element{label: Some(Label::ArgDynamicLength(off)), ..}), fa: Some(ot), ..} =>
        {
            if ot.data == VAL_32_B {
                args.set(off, "uint256[]");
            } else if ot.data == VAL_2_B {
                args.set(off, "string");
            }
            if let Some(Label::Arg(ot_off, _)) = ot.label {
                args.mark_not_bool(ot_off);
            }
        }

          StepResult{op: op::LT|op::GT|op::MUL, fa: Some(Element{label: Some(Label::Arg(off, _)), ..}), ..}
        | StepResult{op: op::LT|op::GT|op::MUL, sa: Some(Element{label: Some(Label::Arg(off, _)), ..}), ..} =>
        {
            args.mark_not_bool(off);
        }

          StepResult{op: op::AND, fa: Some(Element{label: Some(Label::Arg(off, dynamic)), ..}), sa: Some(ot), ..}
        | StepResult{op: op::AND, sa: Some(Element{label: Some(Label::Arg(off, dynamic)), ..}), fa: Some(ot), ..} =>
        {
            let v: U256 = U256::from_be_bytes(ot.data);
            if v.is_zero() {
                // pass
            } else if (v & (v + VAL_1)).is_zero() {
                // 0x0000ffff
                let bl = v.bit_len();
                if bl % 8 == 0 {
                    let t = if bl == 160 { "address".to_string() } else { format!("uint{bl}") };
                    args.set(off, &if dynamic { t + "[]" } else { t });
                }
            } else {
                // 0xffff0000
                let v = U256::from_le_bytes(ot.data);
                if (v & (v + VAL_1)).is_zero() {
                    let bl = v.bit_len();
                    if bl % 8 == 0 {
                        let t = format!("bytes{}", bl / 8);
                        args.set(off, &if dynamic { t + "[]" } else { t });
                    }
                }
            }
        }

        StepResult{op: op::ISZERO, fa: Some(Element{label: Some(Label::Arg(off, dynamic)), ..}), ..} =>
        {
            let v = vm.stack.peek_mut()?;
            v.label = Some(Label::IsZeroResult(off, dynamic));
        }

        StepResult{op: op::ISZERO, fa: Some(Element{label: Some(Label::IsZeroResult(off, dynamic)), ..}), ..} =>
        {
            // Detect check for 0 in DIV, it's not bool in that case: ISZERO, ISZERO, PUSH off, JUMPI, JUMPDEST, DIV
            let mut is_bool = true;
            let op = vm.code[vm.pc];
            if let op::PUSH1..=op::PUSH4 = op {
                let n = (op - op::PUSH0) as usize;
                if vm.code[vm.pc + n + 1] == op::JUMPI {
                    let mut arg: [u8; 4] = [0; 4];
                    arg[(4 - n)..].copy_from_slice(&vm.code[vm.pc + 1..vm.pc + 1 + n]);
                    let jumpdest = u32::from_be_bytes(arg) as usize;
                    if jumpdest + 1 < vm.code.len()
                        && vm.code[jumpdest] == op::JUMPDEST
                        && vm.code[jumpdest + 1] == op::DIV
                    {
                        is_bool = false;
                    }
                }
            }
            if is_bool {
                if dynamic {
                    args.set(off, "bool[]");
                } else if !args.not_bool.contains(&off) {
                    args.set(off, "bool");
                }
            }
        }

        StepResult{op: op::SIGNEXTEND, fa: Some(s0), sa: Some(Element{label: Some(Label::Arg(off, dynamic)), ..}), ..} =>
        {
            if s0.data < VAL_32_B {
                let s0: u8 = s0.data[31];
                let t = format!("int{}{}", (s0+1)*8, if dynamic { "[]" } else { "" });
                args.set(off, &t);
            }
        }

        StepResult{op: op::BYTE, sa: Some(Element{label: Some(Label::Arg(off, _)), ..}), ..} =>
        {
            args.set_if(off, "", "bytes32");
        }

        _ => {}
    };
    Ok(())
}

/// Extracts function arguments
///
/// # Arguments
///
/// * `code` - A slice of deployed contract bytecode
/// * `selector` - A function selector
/// * `gas_limit` - Maximum allowed gas usage; set to `0` to use defaults
///
/// # Examples
///
/// ```
/// use evmole::function_arguments;
/// use hex::decode;
///
/// let code = decode("6080604052348015600e575f80fd5b50600436106030575f3560e01c80632125b65b146034578063b69ef8a8146044575b5f80fd5b6044603f3660046046565b505050565b005b5f805f606084860312156057575f80fd5b833563ffffffff811681146069575f80fd5b925060208401356001600160a01b03811681146083575f80fd5b915060408401356001600160e01b0381168114609d575f80fd5b80915050925092509256").unwrap();
/// let selector = [0x21, 0x25, 0xb6, 0x5b];
///
/// let arguments: String = function_arguments(&code, &selector, 0);
///
/// assert_eq!(arguments, "uint32,address,uint224");
/// ```

pub fn function_arguments(code: &[u8], selector: &Selector, gas_limit: u32) -> String {
    let mut cd: [u8; 32] = [0; 32];
    cd[0..4].copy_from_slice(selector);
    let mut vm = Vm::<Label>::new(
        code,
        Element {
            data: cd,
            label: Some(Label::CallData),
        },
    );
    let mut args = ArgsResult::new();
    let mut gas_used = 0;
    let mut inside_function = false;
    let real_gas_limit = if gas_limit == 0 {
        1e4 as u32
    } else {
        gas_limit
    };
    while !vm.stopped {
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // println!("{}", _e);
                break;
            }
        };
        gas_used += ret.gas_used;
        if gas_used > real_gas_limit {
            break;
        }

        if !inside_function {
            if ret.op == op::EQ || ret.op == op::XOR || ret.op == op::SUB {
                let p = vm.stack.peek().unwrap().data; // unwrap is safe unless we have bug in our evm implementation
                if (ret.op == op::EQ && p == VAL_1_B) || (ret.op != op::EQ && p == VAL_0_B) {
                    if let Some(v) = ret.fa {
                        if v.data[28..32] == vm.calldata.data[0..4] {
                            inside_function = true;
                        }
                    }
                }
            }
            continue;
        }
        // println!("args: {}", args.join_to_string());
        // println!("{:?}\n", vm);

        if analyze(&mut vm, &mut args, ret).is_err() {
            break;
        }
    }

    args.join_to_string()
}
