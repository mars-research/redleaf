#![no_std]

use hashbrown::HashMap;
use core::hash::Hash;
use spin::Mutex;
use libsyscalls::syscalls::sys_get_thread_id;

pub struct ThreadLocal<T> {
    values: Mutex<HashMap<u64, T>>,
    init: fn() -> T,
}

impl<T> ThreadLocal<T> {
    pub fn new(init: fn() -> T) -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            init
        }
    }

    pub fn with<F, R>(&self, f: F) -> R where F: FnOnce(&mut T) -> R {
        let key = sys_get_thread_id();
        let mut values = self.values.lock();
        let value = values.entry(key).or_insert_with(self.init);
        f(value)
    }
}
