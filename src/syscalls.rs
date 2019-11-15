use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use usr::capabilities::Capability;
use usr::syscalls::Syscall;

#[derive(Clone, Copy, PartialEq)]
pub struct UKern {

}

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



impl Syscall for UKern {
    fn sys_print(&self, s: &str) {
        sys_print(s);   
    }

    fn sys_yield(&self) {
        sys_yield();
    }

    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Capability {
        return sys_create_thread(name, func); 
    }

}

