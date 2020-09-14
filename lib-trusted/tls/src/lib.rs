#![no_std]

use hashbrown::HashMap;
use spin::Mutex;
use libsyscalls::syscalls::{sys_current_thread_id, sys_yield};

pub struct ThreadLocal<T> {
    values: Mutex<HashMap<u64, Option<T>>>,
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
        let thread_id = sys_current_thread_id();
        // Take the value out from the global map so we don't hold the lock for too long
        let mut value = self.values.lock().entry(thread_id).or_insert_with(|| Some((self.init)())).take();
        let mut value = value.expect("ThreadLocal object is being used by another thread");

        // Do stuff with the value
        let mut rtn = f(&mut value);

        // Put the value back to the global map
        if self.values.lock().get_mut(&thread_id).unwrap().replace(value).is_some() {
            panic!("This threadlocal variable is accessed by another thread while this thread is using it");
        }

        // Return the result
        rtn
    }

    // drop
    pub fn drop(&self) {
        let thread_id = sys_current_thread_id();
        self.values.lock().remove(&thread_id);
    }
}
