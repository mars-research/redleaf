use crate::rref::RRef;
use libsyscalls;

pub struct RRefArray<T, const N: usize> where T: 'static {
    arr: RRef<[Option<RRef<T>>; N]>
}

impl<T, const N: usize> RRefArray<T, N> {
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
            let domain_id = libsyscalls::syscalls::sys_get_current_domain_id();
            rref.move_to(domain_id);
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
}
