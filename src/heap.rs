use alloc::vec::Vec;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use crate::interrupt::{disable_irq, enable_irq};
use crate::memory::MEM_PROVIDER;

// usize == *mut u8
struct SharedHeapAllocation {
    // TODO: we can *probably* get the domain_id via ptr
    domain_id: u64,
    ptr: usize,
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

    unsafe fn change_domain(&self, ptr: *mut u8, new_domain_id: u64) {
        disable_irq();
        change_domain(ptr, new_domain_id);
        enable_irq();
    }
}

unsafe fn alloc_heap(domain_id: u64, layout: Layout) -> *mut u8 {
    let ptr = MEM_PROVIDER.alloc(layout);
    {
        let lock = alloc_lock.lock();

        unsafe { &mut allocations }.push(SharedHeapAllocation {
            domain_id,
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

fn change_domain(ptr: *mut u8, new_domain_id: u64) {
    // TODO: this is lockless but probably not threadsafe
    // TODO: currently does a linear scan, which is slow
    for allocation in unsafe { &mut allocations }.iter_mut() {
        if ptr == allocation.ptr as *mut u8 {
            allocation.domain_id = new_domain_id;
            break;
        }
    }
}

unsafe fn drop_domain(domain_id: u64) {
    let lock = alloc_lock.lock();
    // remove all allocations from list that belong to the exited domain
    (&mut allocations).retain(|allocation| {
        if domain_id == allocation.domain_id {
            unsafe { MEM_PROVIDER.dealloc(allocation.ptr as *mut u8, allocation.layout) }
            false
        } else {
            true
        }
    });
    drop(lock);
}
