use crate::rref::{RRef, RRefable};

pub struct RRefArray<T: RRefable, const N: usize> where T: 'static {
    arr: RRef<[Option<RRef<T>>; N]>
}

unsafe impl<T: RRefable, const N: usize> RRefable for RRefArray<T, N> {}

impl<T: RRefable, const N: usize> RRefArray<T, N> {
    pub fn new(arr: [Option<RRef<T>>; N]) -> Self {
        Self {
            arr: RRef::new(arr)
        }
    }

    pub fn has(&self, index: usize) -> bool {
        self.arr[index].is_some()
    }

    pub fn get(&mut self, index: usize) -> Option<RRef<T>> {
        let value = self.arr[index].take();
        if let Some(rref) = value.as_ref() {
            unsafe { rref.move_to_current() };
        }
        return value;
    }

    pub fn set(&mut self, index: usize, value: RRef<T>) {
        value.move_to(0); // mark as owned
        self.arr[index].replace(value);
    }

    pub fn move_to(&self, new_domain_id: u64) {
        self.arr.move_to(new_domain_id);
    }

    pub(crate) fn get_ref(&self, index: usize) -> Option<&T> {
        self.arr[index].as_ref().map(|r| &**r)
    }

    pub(crate) fn get_mut(&self, index: usize) -> Option<&mut T> {
        self.arr[index].as_ref().map(|r| {
            unsafe { r.ptr_mut() }
        })
    }
}

impl<T: RRefable, const N: usize> Default for RRefArray<T, N> {
    fn default() -> Self {
        // https://www.joshmcguigan.com/blog/array-initialization-rust/
        let arr = unsafe {
            let mut arr: [Option<RRef<T>>; N] = core::mem::uninitialized();
            for item in &mut arr[..] {
                core::ptr::write(item, None);
            }
            arr
        };
        Self {
            arr: RRef::new(arr)
        }
    }
}
