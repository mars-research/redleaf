#![no_std]
#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(negative_impls)]
#![feature(auto_traits)]
#![feature(specialization)]
#![feature(type_name_of_val)]

extern crate alloc;

mod rref;
mod rref_deque;
mod rref_array;
mod rref_vec;
pub mod traits;
mod owned;

pub use self::rref::init as init;
pub use self::rref::RRef as RRef;
pub use self::rref_array::RRefArray as RRefArray;
pub use self::rref_deque::RRefDeque as RRefDeque;
pub use self::rref_vec::RRefVec as RRefVec;
pub use self::owned::Owned as Owned;

#[cfg(test)]
mod tests {
    use super::*;
    use traits::{RRefable, TypeIdentifiable};
    use alloc::boxed::Box;
    use core::alloc::Layout;
    use alloc::vec::Vec;
    use core::mem;
    use syscalls::{Syscall, Thread, SharedHeapAllocation};
    use hashbrown::HashMap;
    use spin::{Mutex, MutexGuard};

    // Drops the pointer, assumes it is of type T
    fn drop_t<T>(ptr: *mut u8) {
        unsafe {
            CLEANUP_COUNTER += 1;
            // recursively invoke further shared heap deallocation in the tree of rrefs
            let t = core::ptr::read(ptr as *mut T);
            drop(t);
        }
    }

    struct DropMap(HashMap<u64, fn (*mut u8) -> ()>);

    impl DropMap {
        fn add_type<T: 'static + RRefable + TypeIdentifiable> (&mut self) {
            let type_id = T::type_id();
            let type_erased_drop = drop_t::<T>;
            self.0.insert(type_id, type_erased_drop);
        }

        fn get_drop(&self, type_id: u64) -> Option<&fn (*mut u8) -> ()> {
            self.0.get(&type_id)
        }
    }

    pub struct Dropper {
        drop_map: DropMap,
    }

    impl Dropper {
        fn new(drop_map: DropMap) -> Self {
            Self {
                drop_map
            }
        }

        pub fn drop(&self, type_id: u64, ptr: *mut u8) -> bool {
            if let Some(drop_fn) = self.drop_map.get_drop(type_id) {
                (drop_fn)(ptr);
                true
            } else {
                false
            }
        }

        pub fn has_type(&self, type_id: u64) -> bool {
            self.drop_map.get_drop(type_id).is_some()
        }
    }

    struct TestHeap {
        dropper: Dropper,
        map: Mutex<HashMap<usize, syscalls::SharedHeapAllocation>>
    }

    impl TestHeap {
        pub fn new() -> TestHeap {
            let mut drop_map = DropMap(HashMap::new());

            drop_map.add_type::<usize>();
            drop_map.add_type::<RRef<usize>>();
            drop_map.add_type::<Container<usize>>();
            drop_map.add_type::<CleanupTest>();
            drop_map.add_type::<Option<CleanupTest>>();
            drop_map.add_type::<Option<RRef<CleanupTest>>>();
            drop_map.add_type::<Option<RRef<Option<CleanupTest>>>>();
            drop_map.add_type::<[Option<RRef<usize>>; 3]>();
            drop_map.add_type::<[Option<RRef<usize>>; 10]>();
            drop_map.add_type::<[Option<RRef<CleanupTest>>; 4]>();
            drop_map.add_type::<Owner>();


            TestHeap {
                dropper: Dropper::new(drop_map),
                map: Mutex::new(Default::default())
            }
        }
    }

    impl syscalls::Heap for TestHeap {
        unsafe fn alloc(&self, layout: Layout, type_id: u64) -> Option<syscalls::SharedHeapAllocation> {
            if !self.dropper.has_type(type_id) {
                return None;
            }

            let domain_id_pointer = Box::into_raw(Box::<u64>::new(0));
            let borrow_count_pointer = Box::into_raw(Box::<u64>::new(0));

            let mut buf = Vec::with_capacity(layout.size());
            let value_pointer = buf.as_mut_ptr();
            mem::forget(buf);

            let allocation = syscalls::SharedHeapAllocation {
                value_pointer,
                domain_id_pointer,
                borrow_count_pointer,
                layout,
                type_id
            };

            self.map.lock().insert(value_pointer as usize, allocation);

            Some(allocation)
        }

        unsafe fn dealloc(&self, ptr: *mut u8) {
            let allocation = self.map.lock().remove(&(ptr as usize));
            if let Some(allocation) = allocation {
                self.dropper.drop(allocation.type_id, allocation.value_pointer);
            } else {
                panic!("dealloc twice");
            }
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
        fn sys_current_thread_id(&self) -> u64 { 1 }
        fn sys_get_current_domain_id(&self) -> u64 { 1 }
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
        unsafe fn sys_discard_cont(&self) { todo!() }
        fn sys_test_unwind(&self) { todo!() }
    }

    fn init_heap() {
        init(Box::new(TestHeap::new()), 1);
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
                borrow_rref_recursively(borrow_count, rref);
                rref.forfeit();
            }
        };

        let rref = RRef::new(100usize);
        borrow_rref_recursively(0, &rref);
    }

    static mut CLEANUP_COUNTER: usize = 0usize;
    static CLEANUP_LOCK: Mutex<()> = Mutex::new(());
    fn reset_cleanup() -> MutexGuard<'static, ()> {
        let guard = CLEANUP_LOCK.lock();
        unsafe { CLEANUP_COUNTER = 0 };
        guard
    }

    #[derive(Copy, Clone)]
    struct CleanupTest {
        val: usize
    }
    impl TypeIdentifiable for CleanupTest {
        fn type_id() -> u64 {
            1
        }
    }

    #[test]
    fn drop_option_rref() {
        init_heap();
        init_syscall();
        let guard = reset_cleanup();

        let rref = RRef::new(Some(RRef::new(Some(CleanupTest { val: 10 }))));
        assert_eq!(unsafe { CLEANUP_COUNTER }, 0);
        drop(rref);
        // dropping an rref calls cleanup recursively
        assert_eq!(unsafe { CLEANUP_COUNTER }, 2);

        drop(guard);
    }

    #[test]
    fn cleanup_rref_array() {
        init_heap();
        init_syscall();
        let guard = reset_cleanup();

        let mut rref_array = RRefArray::new([
            Some(RRef::new(CleanupTest { val: 10 })),
            Some(RRef::new(CleanupTest { val: 15 })),
            None,
            Some(RRef::new(CleanupTest { val: 20 })),
        ]);
        assert_eq!(unsafe { CLEANUP_COUNTER }, 0);
        drop(rref_array);
        assert_eq!(unsafe { CLEANUP_COUNTER }, 4);

        drop(guard);
    }

    #[test]
    fn cleanup_rref_deque() {
        init_heap();
        init_syscall();
        let guard = reset_cleanup();

        let mut rref_deque = RRefDeque::new([
            Some(RRef::new(CleanupTest { val: 10 })),
            Some(RRef::new(CleanupTest { val: 15 })),
            None,
            Some(RRef::new(CleanupTest { val: 20 })),
        ]);
        assert_eq!(unsafe { CLEANUP_COUNTER }, 0);
        drop(rref_deque);
        assert_eq!(unsafe { CLEANUP_COUNTER }, 4);

        drop(guard);
    }

    // #[test]
    // fn access_rref_vec() {
    //     init_heap();
    //     init_syscall();
    //     let guard = reset_cleanup();
    //
    //     let rref_vec = RRefVec::new(CleanupTest { val: 10 }, 3);
    //     for e in rref_vec.as_slice() {
    //         assert_eq!(e.val, 10);
    //     }
    //
    //     drop(guard);
    // }

    // #[test]
    // fn mutate_rref_vec() {
    //     init_heap();
    //     init_syscall();
    //     let guard = reset_cleanup();
    //
    //     let mut rref_vec = RRefVec::new(CleanupTest { val: 10 }, 3);
    //     for (i, e) in rref_vec.as_mut_slice().iter_mut().enumerate() {
    //         e.val = i;
    //     }
    //
    //     for (i, e) in rref_vec.as_slice().iter().enumerate() {
    //         assert_eq!(i, e.val);
    //     }
    //
    //     drop(guard);
    // }

    // TODO(tianjiao): find a way to test this
    // #[test]
    // fn cleanup_rref_vec() {
    //     init_heap();
    //     init_syscall();
    //     let guard = reset_cleanup();

    //     let a = CleanupTest { val: 10 };
    //     let rref_vec = RRefVec::new(a, 3);
    //     assert_eq!(unsafe { CLEANUP_COUNTER }, 0);
    //     drop(rref_vec);
    //     assert_eq!(unsafe { CLEANUP_COUNTER }, 3);

    //     drop(guard);
    //     drop(a);
    // }

    struct Container<T: 'static + RRefable> {
        inner: RRef<T>,
    }

    impl<T: 'static + RRefable> TypeIdentifiable for Container<T> {
        fn type_id() -> u64 {
            2
        }
    }

    #[test]
    fn cleanup_rref_container() {
        init_heap();
        init_syscall();
        let guard = reset_cleanup();

        let rref = RRef::new(55usize);
        let inner = Container { inner: rref };
        let inner_rref = RRef::new(inner);
        // Container<RRef<Container<RRef<usize>>>>
        let outer = Container { inner: inner_rref };

        assert_eq!(unsafe { CLEANUP_COUNTER }, 0);
        drop(outer);
        assert_eq!(unsafe { CLEANUP_COUNTER }, 2);

        drop(guard);
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

    struct Owner {
        inner: Owned<usize>,
    }

    impl TypeIdentifiable for Owner {
        fn type_id() -> u64 {
            123456789
        }
    }

    #[test]
    fn owned_rref_domain_id() {
        init_heap();
        init_syscall();
        let guard = reset_cleanup();

        let mut owner = RRef::new(Owner {
            inner: Owned::new(RRef::new(0))
        });

        assert_eq!(owner.domain_id(), 1);

        // inner dom_id should be 0, since it is owned by a parent rref
        assert_eq!(owner.inner.rref.as_ref().unwrap().domain_id(), 0);

        // take the inner rref out
        let inner = owner.inner.take().unwrap();

        // inner dom_id should now be 1 (the current domain's id)
        assert_eq!(inner.domain_id(), 1);

        // put it back...
        owner.inner.replace(inner);

        // inner dom_id should be back to 0
        assert_eq!(owner.inner.rref.as_ref().unwrap().domain_id(), 0);

        drop(guard);
    }
}
