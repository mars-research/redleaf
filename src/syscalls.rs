use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use usr::capabilities::Capability;
use syscalls::syscalls::Syscall;

// Print a string 
pub fn sys_print(s: &str) {

    disable_irq();
    println!("{}", s);
    enable_irq(); 
}


// Yield to any thread
pub fn sys_yield() {

    disable_irq();
    println!("sys_yield"); 
    do_yield();
    enable_irq(); 
}

// Create a new thread
pub fn sys_create_thread(name: &str, func: extern fn()) -> Capability  {

    disable_irq();
    println!("sys_create_thread"); 
    let cap = create_thread(name, func);
    enable_irq();
    return cap;
}


pub static UKERN: Syscall = Syscall{
    sys_print,
    sys_yield,
    sys_create_thread,
};

