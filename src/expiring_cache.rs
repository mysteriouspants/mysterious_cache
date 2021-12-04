//! A wrapper for LruCache in which the internal elements can "expire" or become
//! old such that they will not be returned by get anymore.

use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hash},
    time::{Duration, Instant},
};

use crate::cache::Cache;
use crate::lru_cache::LruCache;

/// Wraps a value with the Instant it was inserted at.
struct ExpiringEntry<V> {
    value: V,
    inserted_at: Instant,
}

/// An LruCache which enforces that it will not return values which are older
/// than a given duration. It is important to remember that there is no active
/// eviction mechanism, which is to say that if you populate a cache and leave
/// it alone for the timeout duration, it will still be at capacity. Elements
/// will only evict on expiry if they are accessed past their timeout.
pub struct ExpiringCache<K, V, S = RandomState>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    cache: LruCache<K, ExpiringEntry<V>, S>,
    timeout: Duration,
}

impl<K, V> ExpiringCache<K, V, RandomState>
where
    K: Eq + Hash,
{
    /// Creates a new cache with a given capacity and timeout.
    pub fn with_capacity_and_timeout(
        capacity: usize,
        timeout: Duration,
    ) -> Self {
        Self {
            cache: LruCache::with_capacity(capacity),
            timeout,
        }
    }
}

impl<K, V, S> ExpiringCache<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Creates a new cache with a given capacity, timeout, and hash builder.
    pub fn with_capacity_and_timeout_and_hash_builder(
        capacity: usize,
        timeout: Duration,
        hash_builder: S,
    ) -> Self {
        Self {
            cache: LruCache::with_capacity_and_hash_builder(
                capacity,
                hash_builder,
            ),
            timeout,
        }
    }

    /// Gets the timeout for this cache.
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }

    /// Sets the timeout for this cache. Setting this timeout will not evict any
    /// entries, it will only affect what entries are returned by calls to get
    /// and get_mut.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Gets the time a particular key was inserted into the cache, if present.
    /// Returns Some even if the insertion time is older than the timeout.
    pub fn get_inserted_at<Q>(&mut self, k: &Q) -> Option<Instant>
    where
        Q: Hash + Eq,
    {
        self.cache.get(k).map(|e| e.inserted_at)
    }
}

impl<K, V, S> Cache<K, V> for ExpiringCache<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.cache
            .insert(
                k,
                ExpiringEntry {
                    value: v,
                    inserted_at: Instant::now(),
                },
            )
            .map(|e| e.value)
    }

    fn get_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
    where
        Q: Hash + Eq,
    {
        if let Some(inserted_at) = self.get_inserted_at(k) {
            if inserted_at.elapsed() > self.timeout {
                self.cache.remove(k);
                return None;
            }
        }

        return self.cache.get_mut(k).map(|e| &mut e.value);
    }

    fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Eq,
    {
        self.cache.remove(k).map(|e| e.value)
    }

    fn clear(&mut self) {
        self.cache.clear();
    }

    fn len(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::{Cache, ExpiringCache};

    use super::ExpiringEntry;

    #[test]
    fn readme_snippet() {
        let mut cache: ExpiringCache<u64, u64> =
            ExpiringCache::with_capacity_and_timeout(
                1,
                Duration::from_secs(30),
            );
        // simulate adding something 35 seconds ago
        // this is equivalent to cache.insert(1, 1) followed by sleep(35)
        cache.cache.insert(
            1,
            ExpiringEntry {
                value: 1,
                inserted_at: Instant::now() - Duration::from_secs(35),
            },
        );
        assert_eq!(None, cache.get(&1));
    }
}
