use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
};

use crate::linked_list::{LinkedList, NodeHandle};
use crate::{cache::Cache, null_hasher::BuildNullHasher};

/// Stores an element in the cache with the handle to its position in the
/// eviction queue.
struct StorageNode<V> {
    /// The value being stored.
    value: V,

    /// A handle to this entry's position in the eviction queue.
    q_node: NodeHandle,
}

type KeyHash = u64;

/// A mostly horrible implementation of an LRU Cache, an unholy union of HashMap
/// and Vec. Insertion and retrieval are O(1) operations, as any good LRU cache
/// ought to be, no?
pub struct LruCache<K, V, S = RandomState>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    storage: HashMap<KeyHash, StorageNode<V>, BuildNullHasher>,
    eviction_q: LinkedList<KeyHash>,
    size: usize,
    capacity: usize,
    hash_builder: S,
    // the key is hashed to a u64, so we don't actually store it anywhere. this
    // keeps the cache quite compact, but the expense is that we are incapable
    // of printing back out the contents of the cache except by hash, which is
    // kind of silly.
    kpd: PhantomData<K>,
}

impl<K, V> LruCache<K, V, RandomState>
where
    K: Eq + Hash,
{
    /// Make a new LruCache with a specified capacity, in number of elements.
    pub fn with_capacity(capacity: usize) -> Self {
        LruCache::with_capacity_and_hash_builder(capacity, Default::default())
    }
}

impl<K, V, S> LruCache<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Makes a new LruCache with a specified capacity and hasher.
    pub fn with_capacity_and_hash_builder(capacity: usize, hash_builder: S) -> Self {
        LruCache {
            storage: HashMap::with_capacity_and_hasher(capacity, BuildNullHasher),
            eviction_q: LinkedList::with_capacity(capacity),
            size: 0,
            capacity,
            hash_builder: hash_builder,
            kpd: PhantomData,
        }
    }

    fn hash_k<Q>(&self, k: &Q) -> KeyHash
    where
        Q: Hash + Eq,
    {
        let mut h = self.hash_builder.build_hasher();
        k.hash(&mut h);
        h.finish()
    }
}

impl<K, V, S> Cache<K, V> for LruCache<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        let hash_k = self.hash_k(&k);

        if let Some(storage_node) = self.storage.get(&hash_k) {
            // update the entry if it already exists
            self.eviction_q.remove_node(&storage_node.q_node);
        } else if self.size == self.capacity {
            // evict the least recent addition
            let least_recently_used = self.eviction_q.pop_back().unwrap();
            self.storage.remove(&least_recently_used);
            self.size -= 1;
        }

        let q_node = self.eviction_q.push(hash_k);
        let orig = self
            .storage
            .insert(hash_k, StorageNode { value: v, q_node });
        self.size += 1;

        return orig.map(|node| node.value);
    }

    fn get_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
    where
        Q: Hash + Eq,
    {
        let hash_k = self.hash_k(k);

        let rv = self.storage.get_mut(&hash_k);

        if let Some(storage_node) = rv {
            // remove the node and add it back again to put it at the front of
            // the list. we'll have to store it back in the hashtable as the
            // handle will have changed.
            self.eviction_q.remove_node(&storage_node.q_node);
            storage_node.q_node = self.eviction_q.push(hash_k);
        }

        self.storage.get_mut(&hash_k).map(|sn| &mut sn.value)
    }

    fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Eq,
    {
        let hash_k = self.hash_k(k);

        let rv = self.storage.get(&hash_k);

        if let Some(storage_node) = rv {
            self.eviction_q.remove_node(&storage_node.q_node);
        }

        self.storage.remove(&hash_k).map(|sn| sn.value)
    }

    fn clear(&mut self) {
        self.storage.clear();
        self.eviction_q.clear();
        self.size = 0;
    }

    fn len(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::{Cache, LruCache};
    use crate::null_hasher::BuildNullHasher;

    #[test]
    fn test_cache() {
        // using a nullhasher here makes it a little easier to reason about what
        // key goes to what value should the tests fail. this does mean that the
        // key has to be a u64 or this is liable to fail on 32-bit targets.
        let mut cache: LruCache<u64, u64, BuildNullHasher> =
            LruCache::with_capacity_and_hash_builder(5, BuildNullHasher);

        // fill up the cache
        assert_eq!(None, cache.insert(0, 0));
        assert_eq!(None, cache.insert(1, 1));
        assert_eq!(None, cache.insert(2, 2));
        assert_eq!(None, cache.insert(3, 3));
        assert_eq!(None, cache.insert(4, 4));

        // verify the cache is filled
        assert_eq!(5, cache.storage.len());
        assert_eq!(5, cache.eviction_q.len());

        // push one more thing onto the cache, this will evict "0"
        assert_eq!(None, cache.insert(5, 5));
        // do it twice, just for good measure
        assert_eq!(Some(5), cache.insert(5, 6));

        // verify the cache isn't over capacity
        assert_eq!(5, cache.storage.len());
        assert_eq!(5, cache.eviction_q.len());
        assert_eq!(5, cache.eviction_q.store.len());

        // verify the "1" is still there, which should make it the youngest item
        assert!(cache.get(&1u64).is_some());

        // verify that "2" is now the oldest item and the next to be evicted by
        // putting 6 into the cache
        assert_eq!(None, cache.insert(6, 6));

        assert_eq!(Some(&6), cache.get(&5u64));
        assert_eq!(None, cache.get(&7u64));
    }

    #[test]
    fn readme_snippet() {
        let mut cache: LruCache<usize, String> = LruCache::with_capacity(5);

        cache.insert(0, "Put".to_owned());
        cache.insert(1, "large".to_owned());
        cache.insert(2, "things".to_owned());
        cache.insert(3, "in".to_owned());
        cache.insert(4, "memory".to_owned());
        cache.insert(5, "but not too many".to_owned());

        assert_eq!(None, cache.get(&0));
    }
}
