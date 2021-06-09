extern crate alloc;
use core::panic::PanicInfo;
use spin::Once;
use alloc::boxed::Box;
use syscalls::{Syscall, Thread, Interrupt, Mmap, Continuation, UnwindCause};
use pc_keyboard::{DecodedKey};
use platform::PciBarAddr;

static SYSCALL: Once<Box<dyn Syscall + Send + Sync>> = Once::new();
static INT: Once<Box<dyn Interrupt + Send + Sync>> = Once::new();
static MMAP: Once<Box<dyn Mmap + Send + Sync>> = Once::new();

pub fn init_interrupts(int: Box<dyn Interrupt + Send + Sync>) {
    INT.call_once(|| int);
}

pub fn init_mmap(mmap: Box<dyn Mmap + Send + Sync>) {
    MMAP.call_once(|| mmap);
}

pub fn sys_mmap(bar_addr: &PciBarAddr) {
    let mmap = MMAP.r#try().expect("Mmap system call interface is not initialized.");
    mmap.sys_mmap(bar_addr);
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

pub fn sys_cpuid() -> u32 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_cpuid()
}

pub fn sys_yield() {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_yield();
}

pub fn sys_create_thread(name: &str, func: extern fn()) -> Box<dyn Thread> {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_create_thread(name, func)
}

pub fn sys_current_thread() -> Box<dyn Thread> {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_current_thread()
}

pub fn sys_current_thread_id() -> u64 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_current_thread_id();
}

pub fn sys_get_current_domain_id() -> u64 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_get_current_domain_id();
}

pub unsafe fn sys_update_current_domain_id(new_domain_id: u64) -> u64 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_update_current_domain_id(new_domain_id);
}


pub fn sys_alloc() -> *mut u8 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_alloc()
}

pub fn sys_free(p: *mut u8) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_free(p)
}

pub fn sys_alloc_huge(sz: u64) -> *mut u8 {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_alloc_huge(sz)
}

pub fn sys_free_huge(p: *mut u8) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_free_huge(p)
}

pub fn sys_backtrace() {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_backtrace()
}

pub fn sys_dummy() {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_dummy()
}

pub fn sys_readch_kbd() -> Result<Option<DecodedKey>, &'static str> {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_readch_kbd()
}

pub fn sys_make_condvar() -> syscalls::CondVarPtr {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    scalls.sys_make_condvar()
}

pub unsafe fn sys_register_cont(cont: &Continuation) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_register_cont(cont);
}

pub unsafe fn sys_discard_cont() {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_discard_cont();
}

pub fn sys_unwind(cause: Option<UnwindCause>) {
    let scalls = SYSCALL.r#try().expect("System call interface is not initialized.");
    return scalls.sys_unwind(cause);
}


