use parking_lot::RwLock;
use std::{hash::Hash, marker::PhantomData, sync::Arc};

use crate::cache::Cache;

/// Wrapper for an LruCache which is shareable across thread boundaries.
pub struct SharedCache<C, K, V>(
    Arc<RwLock<C>>,
    PhantomData<K>,
    PhantomData<V>,
)
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

impl<C, K, V> Clone for SharedCache<C, K, V>
where
    C: Cache<K, V>,
    K: Eq + Hash,
    V: Clone,
{
    fn clone(&self) -> Self {
        SharedCache(self.0.clone(), PhantomData, PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::{ExpiringCache, LruCache, SharedCache};

    #[test]
    fn readme_snippet() {
        let cache: SharedCache<LruCache<usize, usize>, usize, usize> =
            SharedCache::with_cache(LruCache::with_capacity(1));
        cache.insert(1, 1);

        let thread_cache = cache.clone();
        let r = thread::spawn(move || thread_cache.get(&1)).join();

        assert_eq!(Some(1), r.unwrap());
    }

    #[test]
    fn expiring_cache_test() {
        let cache: SharedCache<
            ExpiringCache<usize, usize>,
            usize,
            usize,
        > = SharedCache::with_cache(
            ExpiringCache::with_capacity_and_timeout(
                1,
                Duration::from_secs(30),
            ),
        );
        cache.insert(1, 1);
        assert_eq!(Some(1), cache.get(&1));
    }
}
