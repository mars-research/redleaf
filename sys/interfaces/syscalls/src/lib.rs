#![no_std]

extern crate alloc;
use alloc::boxed::Box;

#[derive(Copy, Clone)]
pub struct Syscall {
    pub sys_print: fn(s: &str),
    pub sys_println: fn(s: &str),
    pub sys_yield: fn(),
    pub sys_create_thread: fn(name: &str, func: extern fn()) -> Box<dyn Thread>,
    pub sys_alloc: fn() -> *mut u8,
    pub sys_free: fn(p: *mut u8),
    pub sys_alloc_huge: fn(sz: u64) -> *mut u8,
    pub sys_free_huge: fn(p: *mut u8),
}

pub trait Thread {
    fn set_affinity(&self, affinity: u64);
}
