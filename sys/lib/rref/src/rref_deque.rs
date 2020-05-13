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
}

impl<T, const N: usize> Default for RRefDeque<T, N> {
    fn default() -> Self {
        Self {
            arr: Default::default(),
            head: 0,
            tail: 0,
        }
    }
}

pub struct RRefDequeIter<'a, T: 'a, const N: usize> where T: 'static {
    arr: &'a RRefArray<T, N>,
    curr: usize,
    remaining: usize,
}

impl<'a, T, const N: usize> Iterator for RRefDequeIter<'a, T, N> {
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
