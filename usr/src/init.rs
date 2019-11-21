//extern crate redleaf;

//use redleaf::syscalls::{sys_create_thread, sys_print};
use spin::Once;
use crate::syscalls::{Syscall};

static SYSCALL: Once<Syscall> = Once::new();

pub extern fn init(s: Syscall) {
    SYSCALL.call_once(|| s);
    (s.sys_print)("init userland");
    (s.sys_create_thread)("hello1", hello1); 
    (s.sys_create_thread)("hello2", hello2); 
}


pub extern fn hello1() {
    let s = SYSCALL.r#try().expect("Userland is not initialized.");
    loop {
        (s.sys_print)("hello 1"); 
        (s.sys_yield)();
    }
}

pub extern fn hello2() {
    let s = SYSCALL.r#try().expect("Userland is not initialized.");
    loop {
        (s.sys_print)("hello 2"); 
    }
}

