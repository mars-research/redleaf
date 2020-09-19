use crate::rref_array::RRefArray;
use crate::rref::RRef;
use crate::traits::{RRefable, CustomCleanup, TypeIdentifiable};

pub struct RRefDeque<T: RRefable, const N: usize> where T: 'static {
    arr: RRefArray<T, N>,
    head: usize, // index of the next element that can be written
    tail: usize, // index of the first element that can be read
}

unsafe impl<T: RRefable, const N: usize> RRefable for RRefDeque<T, N> {}

impl<T: RRefable, const N: usize> CustomCleanup for RRefDeque<T, N> {
    fn cleanup(&mut self) {
        #[cfg(features = "rref_dbg")]
        println!("CustomCleanup::{}::cleanup()", core::any::type_name_of_val(self));
        self.arr.cleanup();
    }
}

impl<T: RRefable, const N: usize> RRefDeque<T, N> where [Option<RRef<T>>; N]: TypeIdentifiable {
    pub fn new(empty_arr: [Option<RRef<T>>; N]) -> Self {
        Self {
            arr: RRefArray::new(empty_arr),
            head: 0,
            tail: 0
        }
    }
}

impl<T: RRefable, const N: usize> Default for RRefDeque<T, N> where [Option<RRef<T>>; N]: TypeIdentifiable {
    fn default() -> Self {
        Self {
            arr: Default::default(),
            head: 0,
            tail: 0,
        }
    }
}

impl<T: RRefable, const N: usize> RRefDeque<T, N> {

    // TODO: mark unsafe?
    pub fn move_to(&self, new_domain_id: u64) {
        self.arr.move_to(new_domain_id);
    }

    pub fn borrow(&self) {
        self.arr.borrow();
    }

    pub fn forfeit(&self) {
        self.arr.forfeit();
    }

    pub fn len(&self) -> usize {
        if self.head > self.tail {
            self.head - self.tail
        } else if self.head == self.tail {
            if self.arr.has(self.head) {
                return N
            } else {
                return 0
            }
        } else {
            N - (self.tail - self.head)
        }
    }

    pub fn push_back(&mut self, value: RRef<T>) -> Option<RRef<T>> {
        if self.arr.has(self.head) {
            return Some(value);
        }
        self.arr.set(self.head, value);
        self.head = (self.head + 1) % N;
        return None;
    }

    pub fn pop_front(&mut self) -> Option<RRef<T>> {
        let value = self.arr.get(self.tail);
        if value.is_some() {
            self.tail = (self.tail + 1) % N;
        }
        return value;
    }

    pub fn iter(&self) -> RRefDequeIter<'_, T, N> {
        RRefDequeIter {
            arr: &self.arr,
            curr: self.tail,
            remaining: self.len(),
        }
    }

    pub fn iter_mut(&mut self) -> RRefDequeIterMut<'_, T, N> {
        let len = self.len();
        RRefDequeIterMut {
            arr: &self.arr,
            curr: self.tail,
            remaining: len,
        }
    }
}

pub struct RRefDequeIter<'a, T: 'a + RRefable, const N: usize> where T: 'static {
    arr: &'a RRefArray<T, N>,
    curr: usize,
    remaining: usize,
}

impl<'a, T: RRefable, const N: usize> Iterator for RRefDequeIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.arr.get_ref(self.curr)
            .map(|el| {
                self.curr = (self.curr + 1) % N;
                self.remaining -= 1;
                el
            })
    }
}

pub struct RRefDequeIterMut<'a, T: RRefable, const N: usize> where T: 'static {
    arr: &'a RRefArray<T, N>,
    curr: usize,
    remaining: usize,
}

impl<'a, T: RRefable, const N: usize> Iterator for RRefDequeIterMut<'a, T, N> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.arr.get_mut(self.curr)
            .map(|el| {
                self.curr = (self.curr + 1) % N;
                self.remaining -= 1;
                el
            })
    }
}
