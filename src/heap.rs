use alloc::vec::Vec;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use crate::interrupt::{disable_irq, enable_irq};
use crate::memory::MEM_PROVIDER;

struct SharedHeapAllocation {
    ptr: *mut u8, // *mut T
    domain_id_ptr: *mut u64,
    layout: Layout,
    drop_fn: extern fn(*mut u8) -> (), // semantically Drop::<T>::drop
}

static mut allocations: Vec<SharedHeapAllocation> = Vec::new();
static alloc_lock: Mutex<()> = Mutex::new(());

pub struct PHeap();

impl PHeap {
    pub fn new() -> PHeap {
        PHeap {}
    }
}

impl syscalls::Heap for PHeap {
    unsafe fn alloc(&self, layout: Layout, drop_fn: extern fn(*mut u8) -> ()) -> (*mut u64, *mut u8) {
        disable_irq();
        let ptrs = alloc_heap(layout, drop_fn);
        enable_irq();
        ptrs
    }

    unsafe fn dealloc(&self, ptr: *mut u8) {
        disable_irq();
        dealloc_heap(ptr);
        enable_irq();
    }
}

unsafe fn alloc_heap(layout: Layout, drop_fn: extern fn(*mut u8) -> ()) -> (*mut u64, *mut u8) {
    let domain_id_ptr = MEM_PROVIDER.alloc(Layout::new::<u64>()) as *mut u64;
    let ptr = MEM_PROVIDER.alloc(layout);
    {
        let lock = alloc_lock.lock();

        unsafe { &mut allocations }.push(SharedHeapAllocation {
            ptr,
            domain_id_ptr,
            layout,
            drop_fn,
        });
    }
    (domain_id_ptr, ptr)
}

unsafe fn dealloc_heap(ptr: *mut u8) {
    let lock = alloc_lock.lock();

    // TODO: drop one object, instead of looping through all of them via retain
    (&mut allocations).retain(|allocation | {
        if ptr == allocation.ptr {
            // TODO: drop domain_ptr
            unsafe { MEM_PROVIDER.dealloc(ptr, allocation.layout) }
            false
        } else {
            true
        }
    });

    drop(lock);
}

pub unsafe fn drop_domain(domain_id: u64) {
    let mut lock = Some(alloc_lock.lock());
    // remove all allocations from list that belong to the exited domain
    (&mut allocations).retain(|allocation| {
        let this_domain_id = *(allocation.domain_id_ptr);
        if domain_id == this_domain_id {
            drop(&mut lock.take());
            (allocation.drop_fn)(allocation.ptr);
            lock.replace(alloc_lock.lock());
            // TODO: drop domain_ptr
            unsafe { MEM_PROVIDER.dealloc(allocation.ptr, allocation.layout) }
            false
        } else {
            true
        }
    });
    drop(lock);
}
