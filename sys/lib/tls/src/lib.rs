#![no_std]

use hashbrown::HashMap;
use spin::Mutex;
use libsyscalls::syscalls::sys_current_thread;

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
        let thread_id = sys_current_thread().get_id();
        let mut values = self.values.lock();
        let value = values.entry(thread_id).or_insert_with(self.init);
        f(value)
    }
}
