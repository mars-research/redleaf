use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use usr::capabilities::Capability;
use syscalls::syscalls::Syscall;
use x86::bits64::paging::{PAddr, VAddr};
use crate::arch::vspace::{VSpace, ResourceType};
use crate::memory::paddr_to_kernel_vaddr;
use x86::bits64::paging::BASE_PAGE_SIZE;

macro_rules! round_up {
    ($num:expr, $s:expr) => {
        (($num + $s - 1) / $s) * $s
    };
}

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

pub fn sys_alloc_huge(sz: u64) -> *mut u8 {
    let how_many = round_up!(sz as usize, BASE_PAGE_SIZE as usize) / BASE_PAGE_SIZE;
    disable_irq();
    let paddr: PAddr = VSpace::allocate_pages(how_many, ResourceType::Memory);
    let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
    println!("sys_alloc_huge: returning {:x}", vaddr.as_u64());
    enable_irq();
    vaddr.as_mut_ptr()
}

// todo: implement free!
pub fn sys_free(_p: *mut u8) {
    disable_irq();
    enable_irq();
}

// todo: implement free!
pub fn sys_free_huge(_p: *mut u8) {
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
    sys_alloc_huge,
    sys_free_huge,
};
