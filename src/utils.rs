use crate::{
    evm::{calldata::CallData, op, vm::Vm, U256, VAL_0_B, VAL_1, VAL_1_B},
    DynSolType,
};

macro_rules! match_first_two {
    ($pattern:pat, $other:pat) => {
        [$pattern, $other, ..] | [$other, $pattern, ..]
    };
}

macro_rules! elabel {
    ($label:pat) => {
        Element {
            label: Some($label),
            ..
        }
    };
}

pub(crate) use match_first_two;
pub(crate) use elabel;


/// Executes the EVM until it reaches the start of a function identified by its selector
pub fn execute_until_function_start<T, U>(vm: &mut Vm<T, U>, gas_limit: u32) -> Option<u32>
where
    T: Clone + std::fmt::Debug + std::cmp::Eq,
    U: CallData<T>,
{
    let mut gas_used = 0;
    let mut found = false;
    while !vm.stopped {
        let ret = match vm.step() {
            Ok(v) => v,
            Err(_e) => {
                // println!("{}", _e);
                return None;
            }
        };

        gas_used += ret.gas_used;
        if gas_used > gas_limit {
            return None;
        }

        if found && ret.op == op::JUMPI {
            return Some(gas_used);
        }

        // Look for selector comparison operations
        if matches!(ret.op, op::EQ | op::XOR | op::SUB) {
            let stack_top = vm.stack.peek().expect("always safe unless bug in vm.rs").data;

            let is_selector_match = if ret.op == op::EQ {
                stack_top == VAL_1_B
            } else {
                stack_top == VAL_0_B
            };

            if is_selector_match && ret.args[0].data[28..32] == vm.calldata.selector() {
                found = true;
            }
        }
    }
    None
}

/// Determines the Solidity type based on a bit mask pattern
pub fn and_mask_to_type(mask: U256) -> Option<DynSolType> {
    const ADDRESS_BITS: usize = 160;
    const BITS_PER_BYTE: usize = 8;

    if mask.is_zero() {
        return None;
    }

    // Helper function to check if bit length is byte-aligned
    let is_byte_aligned = |bits: usize| bits % BITS_PER_BYTE == 0;

    // Check for right-aligned mask pattern (0x0000ffff)
    if (mask & (mask + VAL_1)).is_zero() {
        let bit_length = mask.bit_len();
        if is_byte_aligned(bit_length) {
            return Some(if bit_length == ADDRESS_BITS {
                DynSolType::Address
            } else {
                DynSolType::Uint(bit_length)
            });
        }
    }

    // Check for left-aligned mask pattern (0xffff0000)
    let left_aligned_mask = U256::from_le_bytes(mask.to_be_bytes() as [u8; 32]);
    if (left_aligned_mask & (left_aligned_mask + VAL_1)).is_zero() {
        let bit_length = left_aligned_mask.bit_len();
        if is_byte_aligned(bit_length) {
            return Some(DynSolType::FixedBytes(bit_length / BITS_PER_BYTE ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use alloy_primitives::uint;

    use super::*;

    #[test]
    fn test_and_mask_to_type() {
        // Test zero mask
        assert_eq!(and_mask_to_type(U256::ZERO), None);

        // Test address mask
        let address_mask = uint!(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF_U256);
        assert_eq!(and_mask_to_type(address_mask), Some(DynSolType::Address));

        // Test uint masks
        let uint8_mask = uint!(0xFF_U256);
        assert_eq!(and_mask_to_type(uint8_mask), Some(DynSolType::Uint(8)));

        // Test fixed bytes masks
        let bytes2_mask = uint!(0xFFFF_U256) << (256 - 16);
        assert_eq!(and_mask_to_type(bytes2_mask), Some(DynSolType::FixedBytes(2)));

        // Test invalid mask
        let invalid_mask = uint!(0b1010_U256);
        assert_eq!(and_mask_to_type(invalid_mask), None);
    }
}
