use super::{element::Element, U256};
use crate::DynSolType;
use std::error;

pub trait CallData<T> {
    fn load32(&self, offset: U256) -> Element<T>;
    fn load(&self, offset: U256, size: U256)
        -> Result<(Vec<u8>, Option<T>), Box<dyn error::Error>>;
    fn len(&self) -> U256;
    fn selector(&self) -> [u8; 4];
}

/// Describes the type of data being labeled in the calldata.
#[derive(Debug, Copy, Clone)]
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
