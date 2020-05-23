#![no_std]
#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(negative_impls)]
#![feature(optin_builtin_traits)]
#![feature(specialization)]
#![feature(type_name_of_val)]

extern crate alloc;

mod rref;
mod rref_deque;
mod rref_array;
pub mod traits;

pub use self::rref::init as init;
pub use self::rref::RRef as RRef;
pub use self::rref_array::RRefArray as RRefArray;
pub use self::rref_deque::RRefDeque as RRefDeque;

#[cfg(test)]
mod tests {
    use super::*;
    use traits::{RRefable, CustomCleanup};
    use alloc::boxed::Box;
    use core::alloc::Layout;
    use alloc::vec::Vec;
    use core::mem;
    use syscalls::{Syscall, Thread};
    extern crate pc_keyboard;
    use hashbrown::HashMap;
    use spin::Mutex;

    struct TestHeap {
        map: Mutex<HashMap<usize, extern fn(*mut u8)>>,
    }

    impl TestHeap {
        pub fn new() -> TestHeap {
            TestHeap {
                map: Mutex::new(Default::default())
            }
        }
    }

    impl syscalls::Heap for TestHeap {
        unsafe fn alloc(&self, layout: Layout, drop_fn: extern fn(*mut u8) -> ()) -> (*mut u64, *mut u64, *mut u8) {
            let domain_id_ptr = Box::into_raw(Box::<u64>::new(0));
            let borrow_count_ptr = Box::into_raw(Box::<u64>::new(0));

            let mut buf = Vec::with_capacity(layout.size());
            let ptr = buf.as_mut_ptr();
            mem::forget(buf);

            self.map.lock().insert(ptr as usize, drop_fn);

            (domain_id_ptr, borrow_count_ptr, ptr)
        }

        unsafe fn dealloc(&self, ptr: *mut u8) {
            let mut map = self.map.lock();
            // don't call drop_fn here - only in the case of a crashed domain (which we don't simulate)
        }
    }

    pub struct TestSyscall();
    impl TestSyscall {
        pub fn new() -> Self { Self {} }
    }
    #[allow(unused_variables)]
    impl Syscall for TestSyscall {
        fn sys_print(&self, s: &str) {}
        fn sys_println(&self, s: &str) {}
        fn sys_cpuid(&self) -> u32 { 0 }
        fn sys_yield(&self) {}
        fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread> { panic!() }
        fn sys_current_thread(&self) -> Box<dyn Thread> { panic!() }
        fn sys_get_current_domain_id(&self) -> u64 { 0 }
        unsafe fn sys_update_current_domain_id(&self, new_domain_id: u64) -> u64 { 0 }
        fn sys_alloc(&self) -> *mut u8 { panic!() }
        fn sys_free(&self, p: *mut u8) { }
        fn sys_alloc_huge(&self, sz: u64) -> *mut u8 { panic!() }
        fn sys_free_huge(&self, p: *mut u8) {}
        fn sys_backtrace(&self) {}
        fn sys_dummy(&self) {}
        fn sys_readch_kbd(&self) -> core::result::Result<Option<pc_keyboard::DecodedKey>, &'static str> { todo!() }
        fn sys_make_condvar(&self) -> Box<(dyn syscalls::CondVar + Send + Sync + 'static)> { todo!() }
        unsafe fn sys_register_cont(&self, _: &syscalls::Continuation) { todo!() }
        fn sys_test_unwind(&self) { todo!() }
    }

    fn init_heap() {
        init(Box::new(TestHeap::new()), 55);
    }
    fn init_syscall() {
        libsyscalls::syscalls::init(Box::new(TestSyscall::new()));
    }

    #[test]
    fn rref_borrow() {
        init_heap();
        init_syscall();

        fn borrow_rref_recursively(mut borrow_count: u64, rref: &RRef<usize>) {
            assert_eq!(borrow_count, unsafe { *rref.borrow_count_pointer });
            if borrow_count < 10 {
                rref.borrow();
                borrow_count += 1;
                // borrow_rref_recursively(borrow_count, rref);
                rref.forfeit();
            }
        };

        let rref = RRef::new(100usize);
        borrow_rref_recursively(0, &rref);
    }

    #[test]
    fn rref_drop() {
        init_heap();
        init_syscall();

        static mut counter: usize = 0usize;

        struct Container<T: 'static + RRefable> {
            inner: RRef<T>,
        }

        impl<T: 'static + RRefable> CustomCleanup for Container<T> {
            fn cleanup(&mut self) {
                unsafe { counter += 1 };
            }
        }
        impl<T: 'static + RRefable> Drop for Container<T> {
            fn drop(&mut self) {
                self.cleanup();
            }
        }

        let rref = RRef::new(55usize);
        let inner = Container { inner: rref };
        let innerRRef = RRef::new(inner);
        // Container<RRef<Container<RRef<usize>>>>
        let outer = Container { inner: innerRRef };

        unsafe { assert_eq!(counter, 0) };
        drop(outer);
        // should call cleanup on inner and outer
        unsafe { assert_eq!(counter, 2) };
    }

    #[test]
    fn rref_deque_empty() {
        init_heap();
        init_syscall();
        let mut deque = RRefDeque::<usize, 3>::new(Default::default());
        assert!(deque.pop_front().is_none());
    }

    #[test]
    fn rref_deque_insertion() {
        init_heap();
        init_syscall();
        let mut deque = RRefDeque::<usize, 3>::new(Default::default());
        deque.push_back(RRef::new(1));
        deque.push_back(RRef::new(2));
        assert_eq!(deque.pop_front().map(|r| *r), Some(1));
        assert_eq!(deque.pop_front().map(|r| *r), Some(2));
    }

    #[test]
    fn rref_deque_overrite() {
        init_heap();
        init_syscall();
        let mut deque = RRefDeque::<usize, 3>::new(Default::default());
        assert!(deque.push_back(RRef::new(1)).is_none());
        assert!(deque.push_back(RRef::new(2)).is_none());
        assert!(deque.push_back(RRef::new(3)).is_none());
        assert_eq!(deque.push_back(RRef::new(4)).map(|r| *r), Some(4));
        assert_eq!(deque.pop_front().map(|r| *r), Some(1));
        assert!(deque.push_back(RRef::new(5)).is_none());
        assert_eq!(deque.pop_front().map(|r| *r), Some(2));
        assert_eq!(deque.pop_front().map(|r| *r), Some(3));
        assert_eq!(deque.pop_front().map(|r| *r), Some(5));
        assert!(deque.pop_front().is_none());
    }

    #[test]
    fn rref_deque_len() {
        init_heap();
        init_syscall();

        let mut deque = RRefDeque::<usize, 3>::new(Default::default());
        assert_eq!(deque.len(), 0); // h = 0, t = 0

        assert!(deque.push_back(RRef::new(1)).is_none());
        assert_eq!(deque.len(), 1); // h = 1, t = 0

        assert!(deque.push_back(RRef::new(2)).is_none());
        assert_eq!(deque.len(), 2); // h = 2, t = 0

        assert!(deque.push_back(RRef::new(3)).is_none());
        assert_eq!(deque.len(), 3); // h = 0, t = 0

        assert!(deque.push_back(RRef::new(4)).is_some()); // rejected
        assert_eq!(deque.len(), 3); // h = 0, t = 0

        assert_eq!(deque.pop_front().map(|r| *r), Some(1));
        assert_eq!(deque.len(), 2); // h = 0, t = 1

        assert!(deque.push_back(RRef::new(4)).is_none());
        assert_eq!(deque.len(), 3); // h = 1, t = 1

        assert_eq!(deque.pop_front().map(|r| *r), Some(2));
        assert_eq!(deque.len(), 2); // h = 1, t = 2

        assert_eq!(deque.pop_front().map(|r| *r), Some(3));
        assert_eq!(deque.len(), 1); // h = 1, t = 0

        assert_eq!(deque.pop_front().map(|r| *r), Some(4));
        assert_eq!(deque.len(), 0); // h = 1, t = 1
    }

    #[test]
    fn rref_deque_iter() {
        init_heap();
        init_syscall();

        let mut deque = RRefDeque::<usize, 10>::default();

        let mut iter = deque.iter();
        assert_eq!(iter.next(), None);

        for i in 1..=3 {
            deque.push_back(RRef::new(i));
        }

        let mut iter = deque.iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);

        assert_eq!(deque.len(), 3);

        for i in 4..=15 { // 11..=15 dont get added
            deque.push_back(RRef::new(i));
        }

        let mut iter = deque.iter();

        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), Some(&6));
        assert_eq!(iter.next(), Some(&7));
        assert_eq!(iter.next(), Some(&8));
        assert_eq!(iter.next(), Some(&9));
        assert_eq!(iter.next(), Some(&10));
        assert_eq!(iter.next(), None);

        let mut i = 1;
        for n in deque.iter() {
            assert_eq!(&i, n);
            i += 1;
        }
    }

    #[test]
    fn rref_deque_iter_mut() {
        init_heap();
        init_syscall();

        let mut deque = RRefDeque::<usize, 10>::default();

        let mut iter = deque.iter_mut();
        assert_eq!(iter.next(), None);

        for i in 1..=3 {
            deque.push_back(RRef::new(i));
        }

        let mut iter = deque.iter_mut();
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), None);

        assert_eq!(deque.len(), 3);

        for i in 4..=15 { // 11..=15 dont get added
            deque.push_back(RRef::new(i));
        }

        let mut iter = deque.iter_mut();

        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), Some(&mut 4));
        assert_eq!(iter.next(), Some(&mut 5));
        assert_eq!(iter.next(), Some(&mut 6));
        assert_eq!(iter.next(), Some(&mut 7));
        assert_eq!(iter.next(), Some(&mut 8));
        assert_eq!(iter.next(), Some(&mut 9));
        assert_eq!(iter.next(), Some(&mut 10));
        assert_eq!(iter.next(), None);

        let mut i = 1;
        for n in deque.iter_mut() {
            *n = i * 2; // double every element
            i += 1;
        }

        let mut i = 1;
        for n in deque.iter_mut() {
            assert_eq!(&mut (i * 2), n); // check that every element was doubled
            i += 1;
        }
    }
}
