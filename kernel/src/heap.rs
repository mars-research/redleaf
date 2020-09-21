use alloc::vec::Vec;
use alloc::sync::Arc;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use crate::interrupt::{disable_irq, enable_irq};
use crate::memory::MEM_PROVIDER;
use hashbrown::HashMap;

#[derive(Copy, Clone)]
struct SharedHeapAllocation {
    ptr: *mut u8, // *mut T
    domain_id_ptr: *mut u64,
    borrow_count_ptr: *mut u64,
    layout: Layout,
    drop_fn: extern fn(*mut u8) -> (), // semantically Drop::<T>::drop
}

unsafe impl Send for SharedHeapAllocation {}
unsafe impl Sync for SharedHeapAllocation {}

lazy_static! {
    static ref allocations: Mutex<HashMap<usize, SharedHeapAllocation>> = Mutex::new(HashMap::new());
}

pub struct PHeap();

impl PHeap {
    pub fn new() -> PHeap {
        PHeap {}
    }
}

impl syscalls::Heap for PHeap {
    unsafe fn alloc(&self, layout: Layout, drop_fn: extern fn(*mut u8) -> ()) -> (*mut u64, *mut u64, *mut u8) {
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

unsafe fn alloc_heap(layout: Layout, drop_fn: extern fn(*mut u8) -> ()) -> (*mut u64, *mut u64, *mut u8) {
    let domain_id_ptr = MEM_PROVIDER.alloc(Layout::new::<u64>()) as *mut u64;
    let borrow_count_ptr = MEM_PROVIDER.alloc(Layout::new::<u64>()) as *mut u64;
    let ptr = MEM_PROVIDER.alloc(layout);

    unsafe { &mut allocations.lock() }.insert(ptr as usize, SharedHeapAllocation {
        ptr,
        domain_id_ptr,
        borrow_count_ptr,
        layout,
        drop_fn,
    });

    (domain_id_ptr, borrow_count_ptr, ptr)
}

unsafe fn dealloc_heap(ptr: *mut u8) {
    if let Some(allocation) = unsafe { &mut allocations.lock() }.remove(&(ptr as usize)) {
        unsafe {
            MEM_PROVIDER.dealloc(ptr, allocation.layout);
            MEM_PROVIDER.dealloc(allocation.domain_id_ptr as *mut u8, Layout::new::<u64>());
            MEM_PROVIDER.dealloc(allocation.borrow_count_ptr as *mut u8, Layout::new::<u64>());
        }
    }
}

pub unsafe fn drop_domain(domain_id: u64) {
    // remove all allocations from list that belong to the exited domain
    let mut alloc_maps = allocations.lock();
    let mut queue = Vec::<SharedHeapAllocation>::new();
    for key in alloc_maps.keys() {
        let allocation = alloc_maps.get(key).unwrap();
        if *(allocation.domain_id_ptr) == domain_id {
            queue.push(*allocation);
        }
    }
    drop(alloc_maps);
    for allocation in queue.iter() {
        (allocation.drop_fn)(allocation.ptr);

        unsafe {
            MEM_PROVIDER.dealloc(allocation.ptr, allocation.layout);
            MEM_PROVIDER.dealloc(allocation.domain_id_ptr as *mut u8, Layout::new::<u64>());
            MEM_PROVIDER.dealloc(allocation.borrow_count_ptr as *mut u8, Layout::new::<u64>());
        }
    }
}
