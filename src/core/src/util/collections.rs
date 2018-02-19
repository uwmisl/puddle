use std::collections;

pub use std::collections::btree_map::Entry;

/// Map is just a BTreeMap for now because we want deterministic testing.
pub type Map<K, V> = collections::BTreeMap<K, V>;
pub type Set<K> = collections::BTreeSet<K>;
