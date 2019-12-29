#![no_std]

use hashbrown::HashMap;
use core::hash::Hash;

pub trait ThreadLocalKey: Eq + Hash + Sized {
}

impl ThreadLocalKey for u32 {}

pub struct ThreadLocal<K, T> where K: ThreadLocalKey {
    values: HashMap<K, T>,
    init: fn() -> T,
}

impl<K, T> ThreadLocal<K, T> where K: ThreadLocalKey {
    pub fn new(init: fn() -> T) -> Self {
        Self {
            values: HashMap::new(),
            init
        }
    }

    pub fn with<F, R>(&mut self, key: K, f: F) -> R where F: FnOnce(&mut T) -> R {
        f(self.values.entry(key).or_insert_with(self.init))
    }
}
