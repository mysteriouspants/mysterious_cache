pub mod linked_hash_map;
// pub mod linked_btree_map; // TODO: implement

pub use linked_hash_map::LinkedHashMap;

// It would be tempting to try to adapt both BTreeMap and HashMap into a
// single common trait; this is largely an exercise in futility, as the
// two have very different trait bounds (which is probably why they're
// quite separate in the standard library already).

pub type KeyHash = u64;

#[derive(Debug, PartialEq)]
pub struct LinkedMapNode<V> {
    left: Option<KeyHash>,
    value: V,
    right: Option<KeyHash>,
}
