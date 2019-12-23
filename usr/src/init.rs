//extern crate redleaf;

//use redleaf::syscalls::{sys_create_thread, sys_println};
use spin::Once;
use crate::syscalls::{Syscall};
use crate::ls;

static SYSCALL: Once<Syscall> = Once::new();

pub extern fn init(s: Syscall) {
    SYSCALL.call_once(|| s);
    (s.init_fs_temp)();
    ls::ls(&s, "/");
    (s.sys_println)("init userland");
    (s.sys_create_thread)("hello1", hello1); 
    (s.sys_create_thread)("hello2", hello2); 
}


pub extern fn hello1() {
    let s = SYSCALL.r#try().expect("Userland is not initialized.");
    loop {
        (s.sys_println)("hello 1"); 
        (s.sys_yield)();
    }
}

pub extern fn hello2() {
    let s = SYSCALL.r#try().expect("Userland is not initialized.");
    loop {
        (s.sys_println)("hello 2"); 
    }
}

