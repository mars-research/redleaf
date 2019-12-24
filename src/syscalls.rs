use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use syscalls::{Syscall, Thread};
use x86::bits64::paging::{PAddr, VAddr};
use crate::arch::vspace::{VSpace, ResourceType};
use crate::memory::paddr_to_kernel_vaddr;
use x86::bits64::paging::BASE_PAGE_SIZE;
use alloc::boxed::Box; 
use spin::Mutex;
use alloc::sync::Arc; 
use crate::domain::domain::{Domain}; 
//use crate::domain::domain::BOOTING_DOMAIN; 

extern crate syscalls; 

macro_rules! round_up {
    ($num:expr, $s:expr) => {
        (($num + $s - 1) / $s) * $s
    };
}

//pub static BOOT_SYSCALL: BootSyscall = BootSyscall {
//    sys_boot_syscall,
//};

//// AB: I was not able to pass Box<dyn Syscall> as an argument 
//// to user_ep() (maybe it's possible, I didn't have time to 
//// figure it out
//pub fn sys_boot_syscall() -> Box<dyn Syscall> {
//    let pdom = BOOTING_DOMAIN.replace(None);
//
//    enable_irq(); 
//    return pdom.unwrap();
//}

pub struct PDomain {
    dom: Arc<Mutex<Domain>>
}

impl PDomain {
    pub const fn new(dom: Arc<Mutex<Domain>>) -> PDomain {
        PDomain {
            dom: dom,
        }
    }
}

impl syscalls::Syscall for PDomain {

    // Print a string 
    fn sys_print(&self, s: &str) {
        disable_irq();
        print!("{}", s);
        enable_irq(); 
    }
    
    // Print a string and a newline
    fn sys_println(&self, s: &str) {
        disable_irq();
        println!("{}", s);
        enable_irq(); 
    }

    fn sys_alloc(&self) -> *mut u8 {
        disable_irq();
        let paddr: PAddr = VSpace::allocate_one_page();
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        println!("sys_alloc: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    fn sys_alloc_huge(&self, sz: u64) -> *mut u8 {
        let how_many = round_up!(sz as usize, BASE_PAGE_SIZE as usize) / BASE_PAGE_SIZE;
        disable_irq();
        let paddr: PAddr = VSpace::allocate_pages(how_many, ResourceType::Memory);
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        println!("sys_alloc_huge: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    // todo: implement free!
    fn sys_free(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // todo: implement free!
    fn sys_free_huge(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // Yield to any thread
    fn sys_yield(&self) {

        disable_irq();
        println!("sys_yield"); 
        do_yield();
        enable_irq(); 
    }

    // Create a new thread
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>  {

        disable_irq();
        println!("sys_create_thread"); 
        let t = create_thread(name, func);
        enable_irq();
        return t;
    }
}


