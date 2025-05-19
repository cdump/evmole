type HashBuilder = alloy_primitives::map::foldhash::fast::FixedState;

pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V, HashBuilder>;
pub(crate) type HashSet<K> = std::collections::HashSet<K, HashBuilder>;
pub(crate) type IndexMap<K, V> = indexmap::IndexMap<K, V, HashBuilder>;
