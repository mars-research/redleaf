//! Module implementing a simple FNV Hasher
//! and related utilities needed for handling hashing
//! in an [`Index`] hash table.
//!
//! [`Index`]: struct.Index.html

use core::hash::{BuildHasher, Hash, Hasher};

#[inline]
pub fn fnv<K: Sized>(value: &K) -> u64 {
    let num_bytes = core::mem::size_of::<K>() as isize;

    /*
    if num_bytes == 8 {
        // incorrect fnv
        let address = value as *const _ as *const u64;
        let val: u64 = unsafe { *address };
        (val * 14695981039346656037) >> (64 - 24)
    } else {
    */
        let address = value as *const _ as *const u8;

        let mut state: u64 = 0xcbf29ce484222325;

        for i in 0..num_bytes {
            let byte: u8 = unsafe { *(address.offset(i)) };
            state = state.wrapping_mul(0x100_0000_01b3);
            state ^= byte as u64;
            // print!("{} ", byte);
        }
        // println!("-> {}", state);

        state
    // }
}

/// Hashes a `value` using a specified `hasher_builder`.
///
/// # Example
///
/// ```
/// use index::hash::{make_hash, IndexHasherBuilder};
///
/// let val = String::from("Hash this !");
/// let hasher_builder = IndexHasherBuilder;
///
/// let hashed = make_hash(&hasher_builder, &val);
///
/// assert_eq!(hashed, 0xf1b59cbd9867ed1);
/// ```
#[inline]
pub fn make_hash<K: Hash + ?Sized>(hasher_builder: &impl BuildHasher, value: &K) -> u64 {
    let mut hasher = hasher_builder.build_hasher();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Simple hasher using the 64-bit [FNV-1 hash function](https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function)
/// with 64-bit FNV offset basis: `0xcbf29ce484222325`
/// and 64-bit FNV prime: `0x100000001b3`.
#[derive(Debug)]
pub struct IndexHasher {
    state: u64,
}

impl IndexHasher {
    pub fn new() -> IndexHasher {
        IndexHasher {
            state: 0xcbf2_9ce4_8422_2325,
        }
    }
}

impl Hasher for IndexHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.state = self.state.wrapping_mul(0x100_0000_01b3);
            self.state ^= u64::from(*byte);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

impl Default for IndexHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for IndexHasher {
    fn clone(&self) -> Self {
        IndexHasher { state: self.state }
    }
}

/// Builder for [`IndexHasher`].
///
/// [`IndexHasher`]: struct.IndexHasher.html
#[derive(Debug, Clone, Copy)]
pub struct IndexHasherBuilder;

impl BuildHasher for IndexHasherBuilder {
    type Hasher = IndexHasher;

    fn build_hasher(&self) -> IndexHasher {
        IndexHasher::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_string() {
        let val = String::from("Hash this !");
        let hasher_builder = IndexHasherBuilder;

        let hashed = make_hash(&hasher_builder, &val);

        assert_eq!(hashed, 0xf1b59cbd9867ed1);
    }
}
