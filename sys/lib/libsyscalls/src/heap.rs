extern crate alloc;
use spin::Once;
use alloc::boxed::Box;
use core::alloc::Layout;
use syscalls::Heap;

pub static HEAP: Once<Box<dyn Heap + Send + Sync>> = Once::new();

pub fn init(heap: Box<dyn Heap + Send + Sync>) {
    HEAP.call_once(|| heap);
}

pub fn sys_heap_alloc(domain_id: u64, layout: Layout) -> *mut u8 {
    let heap = HEAP.force_get();//.r#try().expect("Heap interface is not initialized.");
    heap.alloc(domain_id, layout)
}

pub fn sys_heap_dealloc(domain_id: u64, ptr: *mut u8, layout: Layout) {
    let heap = HEAP.force_get();//.r#try().expect("Heap interface is not initialized.");
    heap.dealloc(domain_id, ptr, layout);
}

pub fn sys_change_domain(from_domain_id: u64, to_domain_id: u64, ptr: *mut u8, layout: Layout) {
    let heap = HEAP.force_get();//.r#try().expect("Heap interface is not initialized.");
    heap.change_domain(from_domain_id, to_domain_id, ptr, layout);
}

pub fn sys_get_current_domain_id() -> u64 {
    let heap = HEAP.force_get();//.r#try().expect("Heap interface is not initialized.");
    heap.get_current_domain_id()
}

pub fn sys_update_current_domain_id(new_domain_id: u64) -> u64 {
    let heap = HEAP.force_get();//.r#try().expect("Heap interface is not initialized.");
    heap.update_current_domain_id(new_domain_id)
}
