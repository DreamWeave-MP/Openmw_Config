// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

//! The hasher behind the lookup indexes: `FxHash` (rustc's), a multiply-rotate over machine
//! words. The keys are file names and setting keys from the user's own configuration, so they
//! need speed on short strings, not the resistance to crafted collisions `SipHash` pays for.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub(crate) struct FxHasher(u64);

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // The length keeps a zero-padded tail distinct from the bytes it pads.
        self.add(bytes.len() as u64);
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.add(u64::from_le_bytes(*word));
        }
        if !rest.is_empty() {
            let mut word = [0u8; 8];
            word[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(word));
        }
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type FxBuildHasher = BuildHasherDefault<FxHasher>;
pub(crate) type FxHashSet<T> = HashSet<T, FxBuildHasher>;
pub(crate) type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, Hash};

    fn hash_of<T: Hash>(value: &T) -> u64 {
        FxBuildHasher::default().hash_one(value)
    }

    #[test]
    fn equal_keys_hash_equal_and_lengths_matter() {
        assert_eq!(
            hash_of(&"Morrowind.esm"),
            hash_of(&String::from("Morrowind.esm"))
        );
        assert_ne!(hash_of(&"a"), hash_of(&"a\0"));
        assert_ne!(hash_of(&"abcdefgh"), hash_of(&"abcdefghi"));
        let mut set = FxHashSet::default();
        assert!(set.insert("Tribunal.esm".to_owned()));
        assert!(set.contains("Tribunal.esm"));
        assert!(!set.contains("Bloodmoon.esm"));
    }
}
