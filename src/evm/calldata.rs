use super::{element::Element, U256};
use std::error;

pub trait CallData<T> {
    fn load32(&self, offset: U256) -> Element<T>;
    fn load(&self, offset: U256, size: U256)
        -> Result<(Vec<u8>, Option<T>), Box<dyn error::Error>>;
    fn len(&self) -> U256;
    fn selector(&self) -> [u8; 4];
}
