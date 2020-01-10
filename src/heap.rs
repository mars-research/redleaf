use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use crate::interrupt::{disable_irq, enable_irq};
use crate::memory::MEM_PROVIDER;
use syscalls::Heap;

static allocations: Mutex<Vec<(u64, usize, Layout)>> = Mutex::new(Vec::new());

pub struct PHeap();

impl PHeap {
    pub fn new() -> PHeap {
        PHeap {}
    }
}

impl Heap for PHeap {
    fn alloc(&self, domain_id: u64, layout: Layout) -> *mut u8 {
        disable_irq();
        let ptr = alloc_heap(domain_id, layout);
        enable_irq();
        ptr
    }

    fn dealloc(&self, domain_id: u64, ptr: *mut u8, layout: Layout) {
        disable_irq();
        dealloc_heap(domain_id, ptr, layout);
        enable_irq();
    }

    fn change_domain(&self, from_domain_id: u64, to_domain_id: u64, ptr: *mut u8, layout: Layout) {
        disable_irq();
        change_domain(from_domain_id, to_domain_id, ptr, layout);
        enable_irq();
    }
}

fn alloc_heap(domain_id: u64, layout: Layout) -> *mut u8 {
    let ptr = unsafe { MEM_PROVIDER.alloc(layout) };
    allocations.lock().push((domain_id, ptr as usize, layout));
    ptr
}

fn dealloc_heap(domain_id: u64, ptr: *mut u8, layout: Layout) {
    allocations.lock().retain(|(a_domain_id, a_ptr, a_layout) | {
        // only dealloc if it's been allocated in the same way and owned by the calling domain
        // TODO: track domain owner changes
        if domain_id == *a_domain_id && ptr == *a_ptr as *mut u8 && layout == *a_layout {
            unsafe { MEM_PROVIDER.dealloc(ptr, layout) }
            false
        } else {
            true
        }
    });
}

fn change_domain(from_domain_id: u64, to_domain_id: u64, ptr: *mut u8, layout: Layout) {
    allocations.lock().iter_mut().map(|(a_domain_id, a_ptr, a_layout)| {
        if from_domain_id == *a_domain_id && ptr == *a_ptr as *mut u8 && layout == *a_layout {
            *a_domain_id = to_domain_id;
        }
    });
}
