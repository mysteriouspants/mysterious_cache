use super::{KeyHash, LinkedMapNode};
use crate::null_hasher::BuildNullHasher;
use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

/// A layer on top of [`HashMap`] that internally links nodes together
/// so they can be iterated over in insertion order.
pub struct LinkedHashMap<K, V, S = RandomState>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    hash_builder: S,
    interior_map: HashMap<KeyHash, LinkedMapNode<V>, BuildNullHasher>,
    head: Option<KeyHash>,
    tail: Option<KeyHash>,
    kpd: PhantomData<K>,
}

pub struct LinkedHashMapIter<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    idx: Option<KeyHash>,
    inner_map: &'a LinkedHashMap<K, V, S>,
}

pub struct ReverseLinkedHashMapIter<'z, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    idx: Option<KeyHash>,
    inner_map: &'z LinkedHashMap<K, V, S>,
}

impl<K, V> LinkedHashMap<K, V, RandomState>
where
    K: Eq + Hash,
{
    #[allow(unused)] // just leaving this here for completeness' sake
    pub fn with_capacity(capacity: usize) -> Self {
        LinkedHashMap::with_capacity_and_hash_builder(
            capacity,
            Default::default(),
        )
    }
}

impl<K, V, S> LinkedHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    pub fn with_capacity_and_hash_builder(
        capacity: usize,
        hash_builder: S,
    ) -> Self {
        Self {
            hash_builder,
            interior_map: HashMap::with_capacity_and_hasher(
                capacity,
                BuildNullHasher,
            ),
            head: None,
            tail: None,
            kpd: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn iter(&self) -> LinkedHashMapIter<'_, K, V, S> {
        LinkedHashMapIter {
            idx: self.head,
            inner_map: self,
        }
    }

    #[cfg(test)]
    pub fn reverse_iter(
        &self,
    ) -> ReverseLinkedHashMapIter<'_, K, V, S> {
        ReverseLinkedHashMapIter {
            idx: self.tail,
            inner_map: self,
        }
    }

    /// Inserts a new node into this map, returning the previous value
    /// at that key.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        #[cfg(test)]
        let original_size = self.len();
        let k_hash = self.k_hash(&k);

        // reconfigure previous head node
        if let Some(k_head) = self.head {
            if let Some(head_node) = self.interior_map.get_mut(&k_head)
            {
                head_node.left = Some(k_hash);
            }
        }

        // insert new node
        let node = LinkedMapNode {
            left: None,
            value: v,
            right: self.head,
        };

        let previous_node = self.interior_map.insert(k_hash, node);
        self.head = Some(k_hash);

        if self.len() == 1 {
            self.tail = Some(k_hash);
        }

        #[cfg(test)]
        {
            assert_eq!(original_size + 1, self.len());
            assert!(self.head.is_some());
            assert!(self.tail.is_some());
            self.continuity_test();
        }

        previous_node.map(|v| v.value)
    }

    #[allow(unused)] // just leaving this here for completeness' sake
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        Q: Hash + Eq,
    {
        self.interior_map.contains_key(&self.k_hash(k))
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&LinkedMapNode<V>>
    where
        Q: Hash + Eq,
    {
        self.interior_map.get(&self.k_hash(&k))
    }

    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        Q: Hash + Eq,
    {
        self.interior_map
            .get_mut(&self.k_hash(&k))
            .map(|n| &mut n.value)
    }

    pub fn remove_tail(&mut self) -> Option<V> {
        if let Some(tail_k) = self.tail {
            if let Some(tail_node) = self.interior_map.remove(&tail_k) {
                self.tail = tail_node.left;

                #[cfg(test)]
                self.continuity_test();

                return Some(tail_node.value);
            } else {
                #[cfg(test)]
                panic!("Tail references a node that doesn't exist")
            }
        }

        None
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Eq,
    {
        #[cfg(test)]
        let original_len = self.len();
        let k_hash = self.k_hash(k);
        if let Some(removed_node) = self.interior_map.remove(&k_hash) {
            // link the nodes on either side together
            if let Some(left_k) = removed_node.left {
                if let Some(left_node) =
                    self.interior_map.get_mut(&left_k)
                {
                    left_node.right = removed_node.right;
                }
            }

            if let Some(right_k) = removed_node.right {
                if let Some(right_node) =
                    self.interior_map.get_mut(&right_k)
                {
                    right_node.left = removed_node.left;
                }
            }

            // link the head to the new head, if applicable
            if Some(k_hash) == self.head {
                self.head = removed_node.right;
            }

            // link the tail to the new tail, if applicable
            if Some(k_hash) == self.tail {
                self.tail = removed_node.left;
            }

            #[cfg(test)]
            {
                assert_eq!(original_len - 1, self.len());

                if self.len() > 0 {
                    assert!(self.head.is_some());
                    assert!(self.tail.is_some());
                }

                self.continuity_test();
            }

            Some(removed_node.value)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.interior_map.clear();
        self.head = None;
        self.tail = None;

        #[cfg(test)]
        {
            assert_eq!(0, self.len());
            self.continuity_test();
        }
    }

    pub fn len(&self) -> usize {
        self.interior_map.len()
    }

    fn k_hash<Q>(&self, k: &Q) -> KeyHash
    where
        Q: Hash + Eq,
    {
        let mut h = self.hash_builder.build_hasher();
        k.hash(&mut h);
        h.finish()
    }

    #[cfg(test)]
    fn continuity_test(&self) {
        let mut count = 0;

        // iterate through the list and make sure it matches the number
        // of elements in the map
        for _item in self.iter() {
            count = count + 1;
            assert!(count <= self.len());
        }

        assert_eq!(self.len(), count);
        count = 0;

        // iterate through the list in reverse and make sure it matches
        // the number of elements in the map
        for _item in self.reverse_iter() {
            count = count + 1;
            assert!(count <= self.len());
        }

        assert_eq!(self.len(), count);
    }
}

impl<'a, K, V, S> Iterator for LinkedHashMapIter<'a, K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    // TODO: Can this be (K, V) like a real map? We'd have to commit to
    // storing the K as well!
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(k_hash) = self.idx {
            if let Some(k_value) = self.inner_map.get(&k_hash) {
                self.idx = k_value.right;
                return Some(&k_value.value);
            }
        }

        None
    }
}

impl<'z, K, V, S> Iterator for ReverseLinkedHashMapIter<'z, K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    type Item = &'z V;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(k_hash) = self.idx {
            if let Some(k_value) = self.inner_map.get(&k_hash) {
                self.idx = k_value.left;
                return Some(&k_value.value);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::LinkedHashMap;
    use crate::null_hasher::BuildNullHasher;

    /// This test adds three elements to the map and removes the middle,
    /// then the head node, to ensure the structure remains consistent
    /// throughout.
    #[test]
    fn test_linked_hash_map_head_removal() {
        let mut linked_hash_map: LinkedHashMap<
            u64,
            u64,
            BuildNullHasher,
        > = LinkedHashMap::with_capacity_and_hash_builder(
            5,
            BuildNullHasher,
        );

        assert_eq!(None, linked_hash_map.insert(0, 0));
        assert_eq!(None, linked_hash_map.insert(1, 1));
        assert_eq!(None, linked_hash_map.insert(2, 2));

        assert!(matches!(linked_hash_map.remove(&1u64), Some(_)));
        assert!(matches!(linked_hash_map.remove(&0u64), Some(_)));
        assert!(matches!(linked_hash_map.remove(&2u64), Some(_)));
    }

    /// This test adds three elements to the map and removes the middle,
    /// then the tail node, to ensure the structure remains consistent
    /// throughout.
    #[test]
    fn test_linked_hash_map_tail_removal() {
        let mut linked_hash_map: LinkedHashMap<
            u64,
            u64,
            BuildNullHasher,
        > = LinkedHashMap::with_capacity_and_hash_builder(
            5,
            BuildNullHasher,
        );

        assert_eq!(None, linked_hash_map.insert(0, 0));
        assert_eq!(None, linked_hash_map.insert(1, 1));
        assert_eq!(None, linked_hash_map.insert(2, 2));

        assert!(matches!(linked_hash_map.remove(&1u64), Some(_)));
        assert!(matches!(linked_hash_map.remove(&2u64), Some(_)));
        assert!(matches!(linked_hash_map.remove(&0u64), Some(_)));
    }
}
