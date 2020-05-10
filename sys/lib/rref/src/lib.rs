#![no_std]
#![feature(array_value_iter)]
#![feature(const_generics)]
extern crate alloc;
use core::ops::{Deref, DerefMut, Drop};
use core::alloc::Layout;
use spin::Once;
use alloc::boxed::Box;
use libsyscalls;

static HEAP: Once<Box<dyn syscalls::Heap + Send + Sync>> = Once::new();
pub fn init(heap: Box<dyn syscalls::Heap + Send + Sync>) {
    HEAP.call_once(|| heap);
}

// Shared heap allocated value, something like Box<SharedHeapObject<T>>
struct SharedHeapObject<T> where T: 'static {
    domain_id: u64,
    value: T,
}

impl<T> Drop for SharedHeapObject<T> {
    fn drop(&mut self) {
        panic!("SharedHeapObject::drop should never be called.");
    }
}

pub struct RRefArray<T, const N: usize> where T: 'static {
    arr: RRef<[Option<RRef<T>>; N]>
}

impl<T, const N: usize> RRefArray<T, N> {
    fn new(arr: [Option<RRef<T>>; N]) -> Self {
        Self {
            arr: RRef::new(arr)
        }
    }

    fn has(&self, index: usize) -> bool {
        self.arr[index].is_some()
    }

    fn get(&mut self, index: usize) -> Option<RRef<T>> {
        let value = self.arr[index].take();
        if let Some(rref) = value.as_ref() {
            let domain_id = libsyscalls::syscalls::sys_get_current_domain_id();
            rref.move_to(domain_id);
        }
        return value;
    }

    fn set(&mut self, index: usize, value: RRef<T>) {
        value.move_to(0); // mark as owned
        self.arr[index].replace(value);
    }

    fn move_to(&self, new_domain_id: u64) {
        self.arr.move_to(new_domain_id);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use core::mem;
    use syscalls::{Syscall, Thread};
    extern crate pc_keyboard;

    pub struct TestHeap();

    impl TestHeap {
        pub fn new() -> TestHeap {
            TestHeap {}
        }
    }

    impl syscalls::Heap for TestHeap {
        unsafe fn alloc(&self, _: u64, layout: Layout) -> *mut u8 {
            let mut buf = Vec::with_capacity(layout.size());
            let ptr = buf.as_mut_ptr();
            mem::forget(buf);
            ptr
        }

        unsafe fn dealloc(&self, _: u64, _: *mut u8, _: Layout) {}

        unsafe fn change_domain(&self, _: u64, _: u64, _: *mut u8, _: Layout) {}
    }

    pub struct TestSyscall();
    impl TestSyscall {
        pub fn new() -> Self { Self {} }
    }
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
        init(Box::new(TestHeap::new()));
    }
    fn init_syscall() {
        libsyscalls::syscalls::init(Box::new(TestSyscall::new()));
    }

    #[test]
    fn rrefdeque_empty() {
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
}

// RRef (remote reference) is an owned reference to an object on shared heap.
// Only one domain can hold an RRef at a single time, so therefore we can "safely" mutate it.
// A global table retains all memory allocated on the shared heap. When a domain dies, all of
//   its shared heap objects are dropped, which gives us the guarantee that RRef's
//   owned reference will be safe to dereference as long as its domain is alive.
pub struct RRef<T> where T: 'static {
    pointer: *mut SharedHeapObject<T>
}

unsafe impl<T> Send for RRef<T> where T: Send {}
unsafe impl<T> Sync for RRef<T> where T: Sync {}

impl<T> RRef<T> {
    pub fn new(value: T) -> RRef<T> {
        // We allocate the shared heap memory by hand. It will be deallocated in one of two cases:
        //   1. RRef<T> gets dropped, and so the memory under it should be freed.
        //   2. The domain owning the RRef dies, and so the shared heap gets cleaned,
        //        and the memory under this RRef is wiped.

        let domain_id = libsyscalls::syscalls::sys_get_current_domain_id();
        let layout = Layout::new::<SharedHeapObject<T>>();
        let memory = unsafe { HEAP.force_get().alloc(domain_id, layout) };

        let pointer = unsafe {
            // reinterpret allocated bytes as this type
            let ptr = core::mem::transmute::<*mut u8, *mut SharedHeapObject<T>>(memory);
            // initialize the memory
            (*ptr).domain_id = domain_id;
            core::ptr::write(&mut (*ptr).value, value);
            ptr
        };

        RRef {
            pointer
        }
    }

    // TODO: mark unsafe so user domain can't call it?
    // TODO: use &mut self?
    pub fn move_to(&self, new_domain_id: u64) {
        // TODO: race here
        unsafe {
            let from_domain = (*self.pointer).domain_id;
            let layout = Layout::new::<SharedHeapObject<T>>();
            HEAP.force_get().change_domain(from_domain, new_domain_id, self.pointer as *mut u8, layout);
            (*self.pointer).domain_id = new_domain_id
        };
    }
}

impl<T> Drop for RRef<T> {
    fn drop(&mut self) {
        unsafe {
            // TODO: is this drop correct? dropping T should only be necessary for cleanup code,
            //       but calling drop may be undefined behavior
            drop(&mut (*self.pointer).value);
            let layout = Layout::new::<SharedHeapObject<T>>();
            HEAP.force_get().dealloc((*self.pointer).domain_id, self.pointer as *mut u8, layout);
        };
    }
}

impl<T> Deref for RRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.pointer).value }
    }
}

impl<T> DerefMut for RRef<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.pointer).value }
    }
}
