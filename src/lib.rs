//! A quick and dirty implementation of an LRU cache.

mod cache;
mod expiring_cache;
mod linked_list;
mod lru_cache;
mod null_hasher;
#[cfg(feature = "shared_cache")]
mod shared_cache;

pub use cache::Cache;
pub use expiring_cache::ExpiringCache;
pub use lru_cache::LruCache;
#[cfg(feature = "shared_cache")]
pub use shared_cache::SharedCache;
