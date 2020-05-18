// Simplified #![no_std] Maglev implementation adapted from https://github.com/flier/rust-maglev
// Copyright 2020 Flier Lu, Zhaofeng Li
//
// Licensed under Apache License 2.0, see:
// https://github.com/flier/rust-maglev/blob/master/LICENSE
//
// Main changes from Flier's version:
// - Uses core::* imports instead of std::*
// - Uses a fixed table size (M) of 65537
// - Uses two different hashing algorithms for offset and skip
//   (See Section 3.4 in Eisenbud. Here we use FNV and XXHash like NetBricks.)
//
// This implementation is not built to be flexible. We do not implement a connection tracking
// table like Eisenbud.
//
// Quick example:
// 
// let m = Maglev::new(&["blmntuu", "Aeimnpprr", "Aeeimnstz"]);
// println!("Selected backend: {}", &m.get("sanctioned"));

use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use core::iter;
use alloc::vec::Vec;

use fnv::FnvHasher;
use twox_hash::XxHash;

const TABLE_SIZE: usize = 655373;

type FnvHashFactory = BuildHasherDefault<FnvHasher>;
type XxHashFactory = BuildHasherDefault<XxHash>;

/// Maglev lookup table
#[derive(Clone)]
pub struct Maglev<N> {
    pub nodes: Vec<N>,
    pub lookup: Vec<isize>,
}

impl<N: Hash + Eq> Maglev<N> {
    /// Creates a `Maglev` lookup table which will use the given hash builder to hash keys.
    pub fn new<I: IntoIterator<Item = N>>(nodes: I) -> Self {
        let nodes = nodes.into_iter().collect::<Vec<_>>();
        let lookup = Self::populate(&nodes);

        Maglev { nodes, lookup }
    }

    #[inline]
    fn hash_offset_skip<Q: Hash + Eq + ?Sized>(
        key: &Q,
        m: usize,
        h1f: &FnvHashFactory,
        h2f: &XxHashFactory,
    ) -> (usize, usize) {
        let mut h1 = h1f.build_hasher();
        let mut h2 = h2f.build_hasher();

        // This replicates Netbricks' setup
        // https://github.com/NetSys/NetBricks/blob/71dfb94beaeac107d7cd359985f9bd66fd223e1b/test/maglev/src/nf.rs#L21
        key.hash(&mut h1);
        let skip = h1.finish() as usize % (m - 1) + 1;

        key.hash(&mut h2);
        let offset = h2.finish() as usize % m;

        (offset, skip)
    }

    fn populate(nodes: &[N]) -> Vec<isize> {
        let m = TABLE_SIZE;
        let n = nodes.len();

        let h1f: FnvHashFactory = Default::default();
        let h2f: XxHashFactory = Default::default();

        let permutation: Vec<Vec<usize>> = nodes
            .iter()
            .map(|node| {
                let (offset, skip) = Self::hash_offset_skip(&node, m, &h1f, &h2f);
                (0..m).map(|i| (offset + i * skip) % m).collect()
            })
            .collect();

        let mut next: Vec<usize> = iter::repeat(0).take(n).collect();
        let mut entry: Vec<isize> = iter::repeat(-1).take(m).collect();

        let mut j = 0;

        while j < m {
            for i in 0..n {
                let mut c = permutation[i][next[i]];

                while entry[c] >= 0 {
                    next[i] += 1;
                    c = permutation[i][next[i]];
                }

                entry[c] = i as isize;
                next[i] += 1;
                j += 1;

                if j == m {
                    break;
                }
            }
        }

        entry
    }

    #[inline]
    pub fn get_index<Q: ?Sized>(&self, key: &Q) -> usize
    where
        Q: Hash + Eq,
    {
        // NetBricks hashes the flow signature with FNV
        let h1f: FnvHashFactory = Default::default();
        let mut h1 = h1f.build_hasher();
        key.hash(&mut h1);
        let hash = h1.finish() as usize;

        self.lookup[hash % self.lookup.len()] as usize
    }

    #[inline]
    pub fn get<Q: ?Sized>(&self, key: &Q) -> &N
    where
        Q: Hash + Eq,
    {
        &self.nodes[self.get_index(key)]
    }
}
