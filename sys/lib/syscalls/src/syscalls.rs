use crate::capabilities::Capability; 
use spin::Once;

static SYSCALL: Once<Syscall> = Once::new();

#[derive(Copy, Clone)]
pub struct Syscall {
    pub sys_print: fn(s: &str),
    pub sys_yield: fn(),
    pub sys_create_thread: fn(name: &str, func: extern fn()) -> Capability,
    pub sys_alloc: fn() -> *mut u8,
    pub sys_free: fn(p: *mut u8),
}

pub fn init(s: Syscall) {
    SYSCALL.call_once(|| s);
}

pub fn sys_print(s: &str) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    (scalls.sys_print)(s);
}

pub fn sys_yield() {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    (scalls.sys_yield)();
}

pub fn sys_create_thread(name: &str, func: extern fn()) -> Capability {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return (scalls.sys_create_thread)(name, func);
}

pub fn sys_alloc() -> *mut u8 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return (scalls.sys_alloc)();
}

pub fn sys_free(p: *mut u8) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return (scalls.sys_free)(p);
}
