use super::{U256, VAL_131072, element::Element};
use crate::DynSolType;
use std::{collections::BTreeMap, error, marker::PhantomData};

pub trait CallData<T> {
    fn load32(&self, offset: U256) -> Element<T>;
    fn load(&self, offset: U256, size: U256)
    -> Result<(Vec<u8>, Option<T>), Box<dyn error::Error>>;
    fn len(&self) -> U256;
    fn selector(&self) -> [u8; 4];
}

/// Describes the type of data being labeled in the calldata.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum CallDataLabelType {
    /// The label represents the offset to dynamic data.
    Offset,

    /// The label represents the length of dynamic data.
    DynLen,

    /// The label represents the actual value of an argument.
    RealValue,
}

pub trait CallDataLabel: Sized {
    fn label(n: usize, tp: &DynSolType, label_type: CallDataLabelType) -> Option<Self>;
}

/// ABI-aware calldata implementation. Encodes argument types and values into offset-based maps so the EVM can load labeled elements from calldata.
#[derive(Debug)]
pub struct CallDataImpl<T> {
    pub selector: [u8; 4],
    arg_types: BTreeMap<usize, (DynSolType, CallDataLabelType)>,
    arg_vals: BTreeMap<usize, U256>,
    _phantom: PhantomData<T>,
}

impl<T> CallDataImpl<T> {
    pub fn new(selector: [u8; 4], arguments: &[DynSolType]) -> Self {
        let (_, types, vals) = encode(arguments);
        Self {
            selector,
            arg_types: BTreeMap::from_iter(types),
            arg_vals: BTreeMap::from_iter(vals),
            _phantom: PhantomData,
        }
    }
}

impl<T: CallDataLabel> CallData<T> for CallDataImpl<T> {
    fn load32(&self, offset: U256) -> Element<T> {
        let mut data = [0; 32];
        let mut label = None;

        if let Ok(off) = usize::try_from(offset) {
            if off < 4 {
                data[..4 - off].copy_from_slice(&self.selector[off..]);
            } else {
                let xoff = off - 4;
                if let Some(val) = self.arg_vals.get(&xoff) {
                    data = val.to_be_bytes();
                }
                if let Some((tp, label_type)) = self.arg_types.get(&xoff) {
                    label = T::label(xoff, tp, *label_type);
                }
            }
        }
        Element { data, label }
    }

    fn load(
        &self,
        offset: U256,
        size: U256,
    ) -> Result<(Vec<u8>, Option<T>), Box<dyn error::Error>> {
        let mut data = vec![0; u8::try_from(size)? as usize]; // max len limited to max_u8
        let mut label = None;

        if let Ok(off) = usize::try_from(offset) {
            if off < 4 {
                let nlen = std::cmp::min(data.len(), 4 - off);
                data[..nlen].copy_from_slice(&self.selector[off..off + nlen]);
            } else {
                let xoff = off - 4;
                if let Some(val) = self.arg_vals.get(&xoff) {
                    //TODO: look to the left to find proper element
                    let word: [u8; 32] = val.to_be_bytes();
                    let n = std::cmp::min(data.len(), word.len());
                    data[..n].copy_from_slice(&word[..n]);
                }
                if let Some((tp, label_type)) = self.arg_types.get(&xoff) {
                    label = T::label(xoff, tp, *label_type);
                }
            }
        }

        Ok((data, label))
    }

    fn selector(&self) -> [u8; 4] {
        self.selector
    }

    fn len(&self) -> U256 {
        VAL_131072
    }
}

fn is_dynamic(ty: &DynSolType) -> bool {
    match ty {
        DynSolType::Bool
        | DynSolType::Int(_)
        | DynSolType::Uint(_)
        | DynSolType::Address
        | DynSolType::FixedBytes(_) => false,
        DynSolType::FixedArray(val, _) => is_dynamic(val),
        DynSolType::Bytes | DynSolType::String | DynSolType::Array(_) => true,
        DynSolType::Tuple(val) => val.iter().any(is_dynamic),
        _ => unreachable!("Unexpected type {:?}", ty),
    }
}

type ArgTypes = Vec<(usize, (DynSolType, CallDataLabelType))>;
type ArgNonZero = Vec<(usize, U256)>;

fn encode(elements: &[DynSolType]) -> (usize, ArgTypes, ArgNonZero) {
    // (offset, type)
    let mut ret_types: ArgTypes = Vec::with_capacity(elements.len());

    // (offset, value)
    let mut ret_nonzero: ArgNonZero = Vec::with_capacity(elements.len());

    let mut off = 0;

    // (offset, type)
    let mut dynamic: Vec<(usize, &DynSolType)> = Vec::with_capacity(elements.len() / 2);

    for ty in elements.iter() {
        if is_dynamic(ty) {
            dynamic.push((off, ty));
            off += 32;
        } else {
            match ty {
                DynSolType::FixedArray(val, sz) => {
                    let (sz_off, sz_types, sz_nonzero) = encode(&vec![*val.clone(); *sz]);
                    ret_types.extend(sz_types.into_iter().map(|(o, v)| (o + off, v)));
                    ret_nonzero.extend(sz_nonzero.into_iter().map(|(o, v)| (o + off, v)));
                    off += sz_off;
                }
                DynSolType::Tuple(val) => {
                    let (sz_off, sz_types, sz_nonzero) = encode(val);
                    ret_types.extend(sz_types.into_iter().map(|(o, v)| (o + off, v)));
                    ret_nonzero.extend(sz_nonzero.into_iter().map(|(o, v)| (o + off, v)));
                    off += sz_off;
                }
                _ => {
                    ret_types.push((off, (ty.clone(), CallDataLabelType::RealValue)));
                    off += 32;
                }
            }
        }
    }

    for (el_off, ty) in dynamic.into_iter() {
        ret_nonzero.push((el_off, U256::from(off)));
        ret_types.push((el_off, (ty.clone(), CallDataLabelType::Offset)));

        match ty {
            DynSolType::Bytes | DynSolType::String => {
                // string "A" (0x41) with len = 1; data is right-padded in 32-byte slot
                ret_nonzero.push((off, U256::from(1)));
                ret_nonzero.push((off + 32, U256::from(0x41) << 248));
                ret_types.push((off, (ty.clone(), CallDataLabelType::DynLen)));
                ret_types.push((off + 32, (ty.clone(), CallDataLabelType::RealValue)));
                off += 64;
            }
            DynSolType::Array(val) => {
                // len = 1
                ret_nonzero.push((off, U256::from(1u64)));
                off += 32;

                let (dyn_off, dyn_ret_types, dyn_ret_nonzero) = encode(&[*val.clone()]);
                ret_types.extend(dyn_ret_types.into_iter().map(|(o, v)| (o + off, v)));
                ret_nonzero.extend(dyn_ret_nonzero.into_iter().map(|(o, v)| (o + off, v)));
                off += dyn_off;
            }
            DynSolType::Tuple(val) => {
                let (dyn_off, dyn_ret_types, dyn_ret_nonzero) = encode(val);
                ret_types.extend(dyn_ret_types.into_iter().map(|(o, v)| (o + off, v)));
                ret_nonzero.extend(dyn_ret_nonzero.into_iter().map(|(o, v)| (o + off, v)));
                off += dyn_off;
            }
            DynSolType::FixedArray(val, sz) => {
                let (dyn_off, dyn_ret_types, dyn_ret_nonzero) = encode(&[*val.clone()]);
                let data_start = 32 * sz;
                for i in 0..*sz {
                    ret_nonzero.push((off, U256::from(data_start + i * (dyn_off - 32))));
                    off += 32;
                }
                for _ in 0..*sz {
                    ret_types.extend(dyn_ret_types.iter().map(|(o, v)| (o + off - 32, v.clone())));
                    ret_nonzero.extend(
                        dyn_ret_nonzero
                            .iter()
                            .skip(1)
                            .map(|(o, v)| (o + off - 32, *v)),
                    );
                    off += dyn_off - 32;
                }
            }
            _ => panic!("Unexpected type {ty:?}"),
        }
    }

    (off, ret_types, ret_nonzero)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn encode_maps(
        elements: &[DynSolType],
    ) -> (
        usize,
        BTreeMap<usize, (DynSolType, CallDataLabelType)>,
        BTreeMap<usize, U256>,
    ) {
        let (size, types, vals) = encode(elements);
        (size, BTreeMap::from_iter(types), BTreeMap::from_iter(vals))
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct NoLabel;
    impl CallDataLabel for NoLabel {
        fn label(_: usize, _: &DynSolType, _: CallDataLabelType) -> Option<Self> {
            None
        }
    }

    // --- string / bytes encoding ---

    #[test]
    fn encode_string_slot_layout() {
        // layout: [head: offset=32] [len=1] [data="A" right-padded]
        let (size, types, vals) = encode_maps(&[DynSolType::String]);
        assert_eq!(size, 96);

        assert_eq!(vals[&0], U256::from(32u64)); // offset ptr → data area
        assert_eq!(types[&0].1, CallDataLabelType::Offset);

        assert_eq!(vals[&32], U256::from(1u64)); // length = 1
        assert_eq!(types[&32].1, CallDataLabelType::DynLen);

        // "A" must be right-padded: 0x4100...00, not left-padded 0x000...41
        assert_eq!(vals[&64], U256::from(0x41u64) << 248);
        assert_eq!(types[&64].1, CallDataLabelType::RealValue);
    }

    #[test]
    fn load32_string_data_is_right_padded() {
        let cd = CallDataImpl::<NoLabel>::new([0; 4], &[DynSolType::String]);
        // data slot is at calldata offset 4 (selector) + 64 (two words into args)
        let elem = cd.load32(U256::from(4 + 64));
        assert_eq!(elem.data[0], 0x41, "first byte must be 0x41");
        assert_eq!(&elem.data[1..], &[0u8; 31], "rest must be zero-padded");
    }

    #[test]
    fn load_string_data_respects_size() {
        let cd = CallDataImpl::<NoLabel>::new([0; 4], &[DynSolType::String]);
        let base = U256::from(4 + 64); // calldata offset of data slot

        let (data, _) = cd.load(base, U256::from(1)).unwrap();
        assert_eq!(data, [0x41]);

        let (data, _) = cd.load(base, U256::from(4)).unwrap();
        assert_eq!(data, [0x41, 0, 0, 0]);
    }

    // --- nested static FixedArray ---

    #[test]
    fn encode_nested_fixed_array_size() {
        // uint256[2][3] = FixedArray(FixedArray(uint256, 2), 3) → 6 slots, 192 bytes
        let ty: DynSolType = "uint256[2][3]".parse().unwrap();
        let (size, types, vals) = encode_maps(&[ty]);
        assert_eq!(size, 192);
        assert_eq!(types.len(), 6);
        assert!(vals.is_empty());
        for i in 0..6usize {
            assert!(
                types.contains_key(&(i * 32)),
                "missing slot at offset {}",
                i * 32
            );
        }
    }

    // --- nested static Tuple ---

    #[test]
    fn encode_nested_tuple_size() {
        // (uint256, uint256[2]) → 1 + 2 = 3 slots, 96 bytes
        let ty: DynSolType = "(uint256, uint256[2])".parse().unwrap();
        let (size, types, vals) = encode_maps(&[ty]);
        assert_eq!(size, 96);
        assert_eq!(types.len(), 3);
        assert!(vals.is_empty());
        for i in 0..3usize {
            assert!(
                types.contains_key(&(i * 32)),
                "missing slot at offset {}",
                i * 32
            );
        }
    }

    #[test]
    fn encode_flat_tuple_unchanged() {
        // (uint256, address) → 2 slots, 64 bytes — baseline to confirm no regression
        let ty: DynSolType = "(uint256, address)".parse().unwrap();
        let (size, types, vals) = encode_maps(&[ty]);
        assert_eq!(size, 64);
        assert_eq!(types.len(), 2);
        assert!(vals.is_empty());
    }
}
