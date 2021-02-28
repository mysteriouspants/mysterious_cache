//! A hasher which just proxies for the u64 it's given. Used in LruCache, which
//! pre-hashes its keys - so hashing them again in its storage hashmap makes
//! little sense.

use std::{
    hash::{BuildHasher, Hasher},
};

/// Proxies u64's for themselves.
pub(crate) struct NullHasher(u64);

impl Hasher for NullHasher {
    fn write(&mut self, bytes: &[u8]) {
        assert_eq!(8, bytes.len()); // only accept u64's
        for byte in bytes.iter().rev() {
            self.0 = (self.0 << 8) | *byte as u64;
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Builds new NullHashers on demand.
pub(crate) struct BuildNullHasher;

impl BuildHasher for BuildNullHasher {
    type Hasher = NullHasher;

    fn build_hasher(&self) -> Self::Hasher {
        NullHasher(0)
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hasher;

    use super::NullHasher;

    #[test]
    fn test_hasher() {
        let mut h0 = NullHasher(0);
        h0.write_u64(0xc8c8c8c8);
        assert_eq!(0xc8c8c8c8, h0.finish());

        let mut h1 = NullHasher(0);
        h1.write_u64(0xc8c8c8c8c8c8c8c8);
        assert_eq!(0xc8c8c8c8c8c8c8c8, h1.finish());
    }
}
