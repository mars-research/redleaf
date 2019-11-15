//extern crate redleaf;

//use redleaf::syscalls::{sys_create_thread, sys_print};
use crate::syscalls::{Syscall};

pub extern fn init(s: impl Syscall) {
    s.sys_print("init userland");
    s.sys_create_thread("hello1", hello1); 
    s.sys_create_thread("hello2", hello2); 
}


pub extern fn hello1() {
    loop {
        //sys_print("hello 1"); 
        //sys_yield();
    }
}

pub extern fn hello2() {
    loop {
        //sys_print("hello 2"); 
    }
}

