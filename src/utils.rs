use crate::evm::{
    op,
    vm::Vm,
    VAL_1_B,
    VAL_0_B,
};

// Executes the EVM until the start of a function is reached (vm.calldata selector)
pub fn execute_until_function_start<T>(vm: &mut Vm<T>, gas_limit: u32) -> Option<u32>
where
    T: Clone + std::fmt::Debug + std::cmp::Eq + std::hash::Hash
{
    let mut gas_used = 0;
    let mut found = false;
    while !vm.stopped {
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

        if found && ret.op == op::JUMPI {
            return Some(gas_used)
        }

        if ret.op == op::EQ || ret.op == op::XOR || ret.op == op::SUB {
            let p = vm.stack.peek().expect("always safe unless bug in vm.rs").data;
            if (ret.op == op::EQ && p == VAL_1_B) || (ret.op != op::EQ && p == VAL_0_B) {
                if let Some(v) = ret.fa {
                    if v.data[28..32] == vm.calldata.data[0..4] {
                        found = true;
                    }
                }
            }
        }
    }
    None
}
