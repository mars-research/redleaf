use alloc::vec::Vec;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use crate::interrupt::{disable_irq, enable_irq};
use crate::memory::MEM_PROVIDER;

struct SharedHeapAllocation {
    ptr: usize, // *mut SharedHeapObject<T>
    layout: Layout,
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
    unsafe fn alloc(&self, domain_id: u64, layout: Layout) -> *mut u8 {
        disable_irq();
        let ptr = alloc_heap(domain_id, layout);
        enable_irq();
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8) {
        disable_irq();
        dealloc_heap(ptr);
        enable_irq();
    }
}

unsafe fn alloc_heap(domain_id: u64, layout: Layout) -> *mut u8 {
    let ptr = MEM_PROVIDER.alloc(layout);
    {
        let lock = alloc_lock.lock();

        unsafe { &mut allocations }.push(SharedHeapAllocation {
            ptr: ptr as usize,
            layout
        });

        drop(lock);
    }
    ptr
}

unsafe fn dealloc_heap(ptr: *mut u8) {
    let lock = alloc_lock.lock();

    // TODO: drop one object, instead of looping through all of them via retain
    (&mut allocations).retain(|allocation | {
        if ptr == allocation.ptr as *mut u8 {
            unsafe { MEM_PROVIDER.dealloc(ptr, allocation.layout) }
            false
        } else {
            true
        }
    });

    drop(lock);
}

pub unsafe fn drop_domain(domain_id: u64) {
    let lock = alloc_lock.lock();
    // remove all allocations from list that belong to the exited domain
    (&mut allocations).retain(|allocation| {
        // NOTE: allocation.ptr is *mut SharedHeapObject<T>
        //          because SharedHeapObject<T> is repr(C), and domain_id:u64 is the first field,
        //          we can extract domain_id by casting to *const u64 and dereferencing.
        let this_domain_id = *(allocation.ptr as *const u64);
        if domain_id == this_domain_id {
            unsafe { MEM_PROVIDER.dealloc(allocation.ptr as *mut u8, allocation.layout) }
            false
        } else {
            true
        }
    });
    drop(lock);
}
