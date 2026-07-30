use super::Label;
use crate::evm::{U256, calldata::CallData, element::Element};
use std::error;

const SYNTHETIC_SELECTOR: [u8; 4] = [0xaa, 0xbb, 0xcc, 0xdd];

pub(super) struct CallDataImpl {
    len: usize,
    selector: [u8; 4],
}

impl CallDataImpl {
    pub(super) fn new(len: usize) -> Self {
        assert!(len <= 4);
        let mut selector = [0; 4];
        selector[..len].copy_from_slice(&SYNTHETIC_SELECTOR[..len]);
        Self { len, selector }
    }
}

impl CallData<Label> for CallDataImpl {
    fn load32(&self, offset: U256) -> Element<Label> {
        let mut data = [0; 32];
        if offset < U256::from(self.len) {
            let off = usize::try_from(offset).expect("len checked");
            data[..self.len - off].copy_from_slice(&self.selector[off..self.len]);
        }
        Element {
            data,
            label: Some(Label::CallData),
        }
    }

    fn load(
        &self,
        offset: U256,
        size: U256,
    ) -> Result<(Vec<u8>, Option<Label>), Box<dyn error::Error>> {
        let mut data = vec![0; u8::try_from(size)? as usize]; // max len limited to max_u8
        if offset < U256::from(self.len) {
            let off = usize::try_from(offset).expect("len checked");
            let nlen = std::cmp::min(data.len(), self.len - off);
            data[..nlen].copy_from_slice(&self.selector[off..off + nlen]);
        }
        Ok((data, Some(Label::CallData)))
    }

    fn selector(&self) -> [u8; 4] {
        self.selector
    }

    fn len(&self) -> U256 {
        U256::from(self.len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calldata_load32() {
        let cd = CallDataImpl::new(4);

        let mut x = cd.load32(U256::ZERO);
        assert_eq!(x.label, Some(Label::CallData));
        assert!(x.data.starts_with(&[0xaa, 0xbb, 0xcc, 0xdd, 0x00, 0x00]));

        x = cd.load32(U256::from(1));
        assert_eq!(x.label, Some(Label::CallData));
        assert!(x.data.starts_with(&[0xbb, 0xcc, 0xdd, 0x00, 0x00]));

        x = cd.load32(U256::from(2));
        assert_eq!(x.label, Some(Label::CallData));
        assert!(x.data.starts_with(&[0xcc, 0xdd, 0x00, 0x00]));

        x = cd.load32(U256::from(3));
        assert_eq!(x.label, Some(Label::CallData));
        assert!(x.data.starts_with(&[0xdd, 0x00, 0x00]));

        x = cd.load32(U256::from(4));
        assert_eq!(x.label, Some(Label::CallData));
        assert!(x.data.starts_with(&[0x00, 0x00]));

        x = cd.load32(U256::from(64));
        assert_eq!(x.label, Some(Label::CallData));
        assert!(x.data.starts_with(&[0x00, 0x00]));
    }

    #[test]
    fn test_calldata_load() {
        let cd = CallDataImpl::new(4);

        let (mut data, mut label) = cd.load(U256::ZERO, U256::from(5)).unwrap();
        assert_eq!(label, Some(Label::CallData));
        assert_eq!(data, [0xaa, 0xbb, 0xcc, 0xdd, 0x00]);

        (data, label) = cd.load(U256::ZERO, U256::from(3)).unwrap();
        assert_eq!(label, Some(Label::CallData));
        assert_eq!(data, [0xaa, 0xbb, 0xcc]);

        (data, label) = cd.load(U256::from(2), U256::from(4)).unwrap();
        assert_eq!(label, Some(Label::CallData));
        assert_eq!(data, [0xcc, 0xdd, 0x00, 0x00]);

        (data, label) = cd.load(U256::from(2), U256::from(1)).unwrap();
        assert_eq!(label, Some(Label::CallData));
        assert_eq!(data, [0xcc]);

        (data, label) = cd.load(U256::from(4), U256::from(2)).unwrap();
        assert_eq!(label, Some(Label::CallData));
        assert_eq!(data, [0x00, 0x00]);

        (data, label) = cd.load(U256::from(4), U256::from(0)).unwrap();
        assert_eq!(label, Some(Label::CallData));
        assert!(data.is_empty());
    }

    #[test]
    fn test_short_calldata() {
        let cd = CallDataImpl::new(2);
        assert_eq!(cd.len(), U256::from(2));
        assert_eq!(cd.selector(), [0xaa, 0xbb, 0, 0]);

        let data = cd.load32(U256::ZERO);
        assert!(data.data.starts_with(&[0xaa, 0xbb, 0, 0]));

        let data = cd.load32(U256::from(1));
        assert!(data.data.starts_with(&[0xbb, 0, 0]));

        let data = cd.load32(U256::from(2));
        assert!(data.data.starts_with(&[0, 0]));
    }
}
