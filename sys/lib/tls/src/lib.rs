#![no_std]

use hashbrown::HashMap;
use core::hash::Hash;
use spin::Mutex;

pub trait ThreadLocalKey: Eq + Hash + Sized {
}

impl ThreadLocalKey for u32 {}

pub struct ThreadLocal<K, T> where K: ThreadLocalKey {
    values: Mutex<HashMap<K, T>>,
    init: fn() -> T,
}

impl<K, T> ThreadLocal<K, T> where K: ThreadLocalKey {
    pub fn new(init: fn() -> T) -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            init
        }
    }

    pub fn with<F, R>(&self, key: K, f: F) -> R where F: FnOnce(&mut T) -> R {
        f(self.values.lock().entry(key).or_insert_with(self.init))
    }
}
