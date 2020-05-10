use crate::rref_array::RRefArray;
use crate::rref::RRef;

pub struct RRefDeque<T, const N: usize> where T: 'static {
    arr: RRefArray<T, N>,
    head: usize, // index of the next element that can be written
    tail: usize, // index of the first element that can be read
}

impl<T, const N: usize> RRefDeque<T, N> {
    pub fn new(empty_arr: [Option<RRef<T>>; N]) -> Self {
        Self {
            arr: RRefArray::new(empty_arr),
            head: 0,
            tail: 0
        }
    }

    // TODO: mark unsafe?
    pub fn move_to(&self, new_domain_id: u64) {
        self.arr.move_to(new_domain_id);
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
}
