use alloc::vec::Vec;
use core::ops::DerefMut;
use spin::Mutex;

pub struct CrossThreadTempStorage<T> {
    arr: Mutex<Vec<Option<T>>>,
}

impl<T> CrossThreadTempStorage<T> {
    pub fn new() -> Self {
        Self {
            arr: Mutex::new(alloc::vec![]),
        }
    }

    pub fn get(&self, id: usize) -> Option<T> {
        return self.arr.lock().get_mut(id)?.take();
    }

    pub fn put(&self, val: T) -> usize {
        let mut arr = self.arr.lock();
        match arr.iter().position(|item| item.is_none()) {
            Some(id) => {
                arr[id].replace(val);
                id
            },
            None => {
                arr.push(Some(val));
                arr.len() - 1
            }
        }
    }
}


