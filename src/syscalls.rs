use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield};


// Yield system call
pub fn sys_yield() {

    disable_irq();
    println!("sys_yield"); 
    do_yield();
    enable_irq(); 
}


