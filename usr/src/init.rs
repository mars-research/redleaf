extern crate redleaf;

use redleaf::syscalls::{sys_create_thread, sys_print};

pub extern fn init() {
    sys_create_thread("hello1", hello1); 
    sys_create_thread("hello2", hello2); 
}


pub extern fn hello1() {
    loop {
        sys_print("hello 1"); 
        sys_yield();
    }
}

pub extern fn hello2() {
    loop {
        sys_print("hello 2"); 
    }
}

