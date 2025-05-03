use ahash::RandomState;

#[derive(Clone)]
pub(crate) struct MyHashBuilder {
    state: RandomState,
}

impl Default for MyHashBuilder {
    fn default() -> Self {
        Self {
            state: RandomState::with_seed(42),
        }
    }
}
impl std::hash::BuildHasher for MyHashBuilder {
    type Hasher = <RandomState as std::hash::BuildHasher>::Hasher;
    fn build_hasher(&self) -> Self::Hasher {
        self.state.build_hasher()
    }
}

pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V, MyHashBuilder>;
pub(crate) type HashSet<K> = std::collections::HashSet<K, MyHashBuilder>;
pub(crate) type IndexMap<K, V> = indexmap::IndexMap<K, V, MyHashBuilder>;
