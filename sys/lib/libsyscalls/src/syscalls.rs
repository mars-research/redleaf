extern crate alloc;
use spin::Once;
use alloc::boxed::Box;
use syscalls::{Syscall, Thread};

static SYSCALL: Once<Box<dyn Syscall + Send + Sync>> = Once::new();

pub fn init(s: Box<dyn Syscall + Send + Sync>) {
    SYSCALL.call_once(|| s);
}

pub fn sys_print(s: &str) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_print(s);
}

pub fn sys_println(s: &str) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_println(s);
}

pub fn sys_yield() {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_yield();
}

pub fn sys_create_thread(name: &str, func: extern fn()) -> Box<dyn Thread> {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_create_thread(name, func);
}

pub fn sys_alloc() -> *mut u8 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_alloc();
}

pub fn sys_free(p: *mut u8) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_free(p);
}

pub fn sys_alloc_huge(sz: u64) -> *mut u8 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_alloc_huge(sz);
}

pub fn sys_free_huge(p: *mut u8) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_free_huge(p);
}

pub fn sys_get_thread_id() -> u32 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_get_thread_id();
}
