extern crate alloc;
use spin::Once;
use alloc::boxed::Box;
use syscalls::{Syscall, Thread, Interrupt};

static SYSCALL: Once<Box<dyn Syscall + Send + Sync>> = Once::new();
static INT: Once<Box<dyn Interrupt + Send + Sync>> = Once::new();

pub fn init_interrupts(int: Box<dyn Interrupt + Send + Sync>) {
    INT.call_once(|| int);
}

pub fn sys_recv_int(int: u8) {
    let ints = INT.r#try().expect("Interrupt system call interface is not initialized.");
    ints.sys_recv_int(int);
}

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

pub fn sys_current_thread() -> Box<dyn Thread> {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_current_thread();
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
