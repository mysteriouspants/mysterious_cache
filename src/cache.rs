use std::hash::Hash;

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
    fn get<'a, Q>(&'a mut self, k: &Q) -> Option<&'a V>
    where
        Q: Hash + Eq,
    {
        self.get_mut(k).map(|v| {
            let v: &V = v;
            v
        })
    }

    /// Get a mutable reference to an item from the cache. This also makes the
    /// item the youngest item in the cache and the least elegible for eviction.
    fn get_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
    where
        Q: Hash + Eq;

    /// Bust a move, returning whatever was there.
    fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        Q: Hash + Eq;

    /// Clears the cache entirely.
    fn clear(&mut self);

    /// The number of items stored in the cache right now.
    fn len(&self) -> usize;
}
