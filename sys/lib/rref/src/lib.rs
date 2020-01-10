#![no_std]
extern crate alloc;
use core::ops::{Deref, DerefMut, Drop};
use alloc::boxed::Box;
use libsyscalls::heap::{sys_heap_alloc, sys_heap_dealloc, sys_change_domain};
use core::alloc::Layout;

// Shared heap allocated value, something like Box<SharedHeapObject<T>>
pub struct SharedHeapObject<T> where T: 'static + Send {
    pub domain_id: u64,
    pub value: T,
}

impl<T> Drop for SharedHeapObject<T> where T: Send {
    fn drop(&mut self) {
        panic!("SharedHeapObject::drop should never be called.");
    }
}

// RRef (remote reference) has an unowned reference to an object on shared heap.
// Only one domain can hold an RRef at a single time, so therefore we can "safely" mutate it.
// A global table retains all memory allocated on the shared heap. When a domain dies, all of
//   its shared heap objects are dropped, which gives us the guarantee that RRef's
//   unowned reference will be safe to dereference as long as its domain is alive.
pub struct RRef<T> where T: 'static + Send {
    pointer: *mut SharedHeapObject<T>
}

impl<T> RRef<T> where T: Send {
    pub fn new(object: Box<SharedHeapObject<T>>) -> RRef<T> {
        // `object` is a pointer to a value on the shared heap.
        // we consume the box into a raw pointer, so that we can deallocate it under our own rules
        // this will happen in two cases:
        //   1. RRef<T> gets dropped, and so the memory under it should be freed.
        //   2. Domain owning the RRef dies, and so shared heap gets cleaned and the domain's
        //        RRef's (including this one) are dropped.
        RRef {
            pointer: Box::into_raw(object)
        }
    }

    pub fn move_to(&mut self, new_domain_id: u64) {
        // TODO: race here
        unsafe {
            sys_change_domain((*self.pointer).domain_id, new_domain_id, self.pointer as *mut u8, Layout::new::<T>());
            (*self.pointer).domain_id = new_domain_id
        };
    }
}

impl<T> Drop for RRef<T> where T: Send {
    fn drop(&mut self) {
        unsafe {
            sys_heap_dealloc((*self.pointer).domain_id, self.pointer as *mut u8, Layout::new::<T>());
        }
    }
}

impl<T> Deref for RRef<T> where T: Send {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.pointer).value }
    }
}

impl<T> DerefMut for RRef<T> where T: Send {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.pointer).value }
    }
}
