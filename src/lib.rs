//! A quick and dirty implementation of an LRU cache.

use parking_lot::RwLock;
use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
};

/// Alias for the index of a node in the linked list's storage vec.
#[derive(Copy, Clone)]
struct NodeHandle(usize);

/// A node that lives in a linked list.
struct Node<T>
where
    T: Eq + Copy,
{
    /// The value being stored.
    value: T,

    /// The index of the node previous to this one.
    prev: NodeHandle,

    /// The index of the next node in the list.
    next: NodeHandle,
}

/// This is the funniest linked list y'all ever did see. It has a single
/// continguous Vec of nodes, which are addressed by their position in the Vec.
/// It freelists elements that have been removed. It has no compaction, it's
/// designed for use in situations where it will be at a determined size for its
/// whole life, and is optimized for random-access node removal.
///
/// Why? By doing this we can keep the nodes closer together and make it easier
/// for the CPU's cache system to reason about them. It's trying to get the best
/// of O(1) insertion/removal of a LinkedList with the low-level memory handling
/// of an array.
///
/// Leaking a NodeHandle outside the linked list is important for LruCache,
/// which can at any time pull a node out of the eviction queue and put it back
/// in at the front (when something is most recent as opposed to least-recent).
struct LinkedList<T>
where
    T: Eq + Copy,
{
    /// The nodes in the list.
    store: Vec<Node<T>>,

    /// Entries in the list which aren't in use anymore. These will be reused.
    free: Vec<NodeHandle>,

    /// The first node in the list.
    head: Option<NodeHandle>,
}

impl<T> LinkedList<T>
where
    T: Eq + Copy,
{
    /// Creates a new linked list with a specific capacity.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            store: Vec::with_capacity(capacity),
            free: Vec::with_capacity(capacity),
            head: None,
        }
    }

    /// The length of this linked list.
    fn len(&self) -> usize {
        self.store.len() - self.free.len()
    }

    /// Gets an element from the list.
    fn get(&self, node: &NodeHandle) -> Option<&T> {
        self.store.get(node.0).map(|node| &node.value)
    }

    /// Pushes t onto the front of the list and returns a handle to the node.
    fn push(&mut self, t: T) -> NodeHandle {
        let mut n = Node {
            value: t,
            prev: NodeHandle(0),
            next: NodeHandle(0),
        };

        // use the first available location in the storage vec, or infer what
        // the next location will be on push.
        let idx = self
            .free
            .pop()
            .unwrap_or_else(|| NodeHandle(self.store.len()));

        if let Some(head) = self.head {
            // link this node into the chain
            n.prev = self.store[head.0].prev;
            n.next = head;

            self.store[n.prev.0].next = idx;
            self.store[n.next.0].prev = idx;
        }

        self.head = Some(idx);

        if self.store.len() <= idx.0 {
            self.store.push(n);
        } else {
            self.store[idx.0] = n;
        }

        return idx;
    }

    /// Pops the back node off the list if it exists.
    fn pop_back(&mut self) -> Option<T> {
        if let Some(head) = self.head {
            let head_prev = self.store[head.0].prev;
            let prev = self.store[head_prev.0].value;
            self.remove_node(&head_prev);
            return Some(prev);
        }

        return None;
    }

    /// Remove an arbitrary node from the list.
    fn remove_node(&mut self, node: &NodeHandle) {
        if self.len() == 1 {
            // just reset head and freelist the node
            self.head = None;
        } else {
            if node.0 == self.head.unwrap().0 {
                self.head = Some(self.store[node.0].next);
            }

            // link prev to next and next to prev so node doesn't exist in the
            // chain anymore; it'll get overwritten at some later push by
            // placing its handle on the freelist
            let prev = self.store[node.0].prev;
            let next = self.store[node.0].next;

            self.store[prev.0].next = self.store[node.0].next;
            self.store[next.0].prev = self.store[node.0].prev;
        }

        self.free.push(NodeHandle(node.0));
    }

    /// The tail of this linked list.
    fn tail(&self) -> Option<&T> {
        if let Some(head) = self.head {
            let tail = &self.store[self.store[head.0].prev.0];
            return Some(&tail.value);
        }

        None
    }

    /// Clears this linked list. Does not free the underlying buffers.
    fn clear(&mut self) {
        self.store.clear();
        self.free.clear();
        self.head = None;
    }
}

/// Stores an element in the cache with the handle to its position in the
/// eviction queue.
struct StorageNode<V> {
    /// The value being stored.
    value: V,

    /// A handle to this entry's position in the eviction queue.
    q_node: NodeHandle,
}

/// Describes what a cache is.
pub trait Cache<K, V>
where
    K: Eq + Hash,
{
    /// Push a new element into the Cache, which may evict the oldest item if
    /// the cache is at capacity. Returns the previous value in the cache if the
    /// key already had a value there.
    fn insert(&mut self, k: K, v: V) -> Option<V>;

    /// Get an item from the Cache. This also makes the item the youngest item
    /// in the cache and the least eligible for eviction.
    fn get<'a>(&'a mut self, k: &K) -> Option<&'a V>;

    /// Get a mutable reference to an item from the cache. This also makes the
    /// item the youngest item in the cache and the least elegible for eviction.
    fn get_mut<'a>(&'a mut self, k: &K) -> Option<&'a mut V>;

    /// Bust a move, returning whatever was there.
    fn remove(&mut self, k: &K) -> Option<V>;

    /// Clears the cache entirely.
    fn clear(&mut self);

    /// The number of items stored in the cache right now.
    fn len(&self) -> usize;
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
    storage: HashMap<KeyHash, StorageNode<V>>,
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
    pub fn with_capacity_and_hash_builder(capacity: usize, hash_builder: S) -> Self {
        LruCache {
            storage: HashMap::with_capacity(capacity),
            eviction_q: LinkedList::with_capacity(capacity),
            size: 0,
            capacity,
            hash_builder: hash_builder,
            kpd: PhantomData,
        }
    }

    fn hash_k(&self, k: &K) -> KeyHash {
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

    fn get<'a>(&'a mut self, k: &K) -> Option<&'a V> {
        self.get_mut(k).map(|v| {
            let v: &V = v;
            v
        })
    }

    fn get_mut<'a>(&'a mut self, k: &K) -> Option<&'a mut V> {
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

    fn remove(&mut self, k: &K) -> Option<V> {
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

/// Wrapper for an LruCache which is shareable across thread boundaries.
pub struct SharedCache<C, K, V>(Arc<RwLock<C>>, PhantomData<K>, PhantomData<V>)
where
    C: Cache<K, V>,
    K: Eq + Hash,
    V: Clone;

impl<C, K, V> SharedCache<C, K, V>
where
    C: Cache<K, V>,
    K: Eq + Hash,
    V: Clone,
{
    /// Wraps a cache into a shared cache accessor, making it safe to move
    /// across thread boundaries. Enforces an additional constraint of Clone on
    /// values.
    pub fn with_cache(cache: C) -> Self {
        Self(Arc::from(RwLock::from(cache)), PhantomData, PhantomData)
    }

    /// Inserts an item into the cache.
    pub fn insert(&self, k: K, v: V) -> Option<V> {
        self.0.write().insert(k, v)
    }

    /// Get an item from the cache. This clones it to minimize the lock time of
    /// the cache.
    pub fn get(&self, k: &K) -> Option<V> {
        self.0.write().get(k).map(|v| v.clone())
    }

    /// Remove an item from the cache, returning the removed item if it existed.
    pub fn remove(&self, k: &K) -> Option<V> {
        self.0.write().remove(k)
    }

    /// Clears the cache.
    pub fn clear(&self) {
        self.0.write().clear()
    }

    /// The number of elements in the cache at present.
    pub fn len(&self) -> usize {
        self.0.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::{Cache, LruCache};

    #[test]
    fn test_cache() {
        let mut cache: LruCache<usize, usize> = LruCache::with_capacity(5);

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
        assert!(cache.get(&1).is_some());

        // verify that "2" is now the oldest item and the next to be evicted by
        // putting 6 into the cache
        assert_eq!(None, cache.insert(6, 6));

        assert_eq!(Some(&6), cache.get(&5));
        assert_eq!(None, cache.get(&7));
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
