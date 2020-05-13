use libsyscalls;
use alloc::boxed::Box;
use core::ops::{Deref, DerefMut, Drop};
use core::alloc::Layout;
use spin::Once;

static HEAP: Once<Box<dyn syscalls::Heap + Send + Sync>> = Once::new();
pub fn init(heap: Box<dyn syscalls::Heap + Send + Sync>) {
    HEAP.call_once(|| heap);
}

// Shared heap allocated value, something like Box<SharedHeapObject<T>>
// This is the struct allocated on the shared heap.
struct SharedHeapObject<T> where T: 'static {
    domain_id: u64,
    value: T,
}

impl<T> Drop for SharedHeapObject<T> {
    fn drop(&mut self) {
        panic!("SharedHeapObject::drop should never be called.");
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
            HEAP.force_get().change_domain(self.pointer as *mut u8, new_domain_id);
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
            HEAP.force_get().dealloc(self.pointer as *mut u8);
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