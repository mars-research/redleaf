use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use usr::capabilities::Capability;
use syscalls::syscalls::Syscall;
use x86::bits64::paging::{PAddr, VAddr};
use crate::arch::vspace::VSpace;
use crate::memory::paddr_to_kernel_vaddr;

// Print a string 
pub fn sys_print(s: &str) {

    disable_irq();
    println!("{}", s);
    enable_irq(); 
}

pub fn sys_alloc() -> *mut u8 {
    disable_irq();
    let paddr: PAddr = VSpace::allocate_one_page();
    let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
    println!("sys_alloc: returning {:x}", vaddr.as_u64());
    enable_irq();
    vaddr.as_mut_ptr()
}

// TODO: Implement free!
pub fn sys_free(_p: *mut u8) {
    disable_irq();
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
    sys_alloc,
    sys_free,
};
