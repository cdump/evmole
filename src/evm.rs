use alloy_primitives::uint;

pub mod memory;
pub mod op;
pub mod stack;
pub mod vm;

pub use alloy_primitives::U256;

pub const VAL_0_B: [u8; 32] = U256::ZERO.to_be_bytes();

pub const VAL_1: U256 = uint!(1_U256);
pub const VAL_1_B: [u8; 32] = VAL_1.to_be_bytes();

pub const VAL_4: U256 = uint!(4_U256);

pub const VAL_32: U256 = uint!(32_U256);
pub const VAL_32_B: [u8; 32] = VAL_32.to_be_bytes();

pub const VAL_256: U256 = uint!(256_U256);

pub const VAL_1024: U256 = uint!(1024_U256);

pub const VAL_1M: U256 = uint!(1000000_U256);

#[derive(Clone)]
pub struct Element<T> {
    pub data: [u8; 32],
    pub label: Option<T>,
}

impl<T> Element<T>
where
    T: Clone,
{
    fn load(&self, offset: U256, size: usize) -> Element<T> {
        let mut data: [u8; 32] = [0; 32];

        let off32: usize = offset.try_into().unwrap_or(33);
        if off32 < 32 {
            let to = std::cmp::min(off32 + size, 32);
            data[0..(to - off32)].copy_from_slice(&self.data[off32..to]);
        }

        Element {
            data,
            label: self.label.clone(),
        }
    }
}

impl<T> From<Element<T>> for U256 {
    fn from(val: Element<T>) -> Self {
        U256::from_be_slice(&val.data)
    }
}

impl<T> From<&Element<T>> for U256 {
    fn from(val: &Element<T>) -> Self {
        U256::from_be_slice(&val.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_load() {
        let e = Element::<u32> {
            data: [
                1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                7, 8, 9, 10,
            ],
            label: Some(7),
        };

        let r = e.load(U256::ZERO, 3);
        assert_eq!(r.label, Some(7));
        assert_eq!(
            r.data,
            [
                1, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );

        let r = e.load(U256::ZERO, 32);
        assert_eq!(r.label, Some(7));
        assert_eq!(
            r.data,
            [
                1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                7, 8, 9, 10
            ]
        );

        let r = e.load(VAL_1, 3);
        assert_eq!(r.label, Some(7));
        assert_eq!(
            r.data,
            [
                2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );

        let r = e.load(VAL_1, 32);
        assert_eq!(r.label, Some(7));
        assert_eq!(
            r.data,
            [
                2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,
                8, 9, 10, 0
            ]
        );

        let r = e.load(VAL_32, 32);
        assert_eq!(r.label, Some(7));
        assert_eq!(r.data, [0; 32]);
    }
}
