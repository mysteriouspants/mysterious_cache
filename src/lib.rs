//! A quick and dirty implementation of an LRU cache.

mod cache;
mod linked_list;
mod lru_cache;
mod null_hasher;
mod shared_cache;

pub use cache::Cache;
pub use lru_cache::LruCache;
pub use shared_cache::SharedCache;
