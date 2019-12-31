#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_recv_int, sys_get_thread_id};
use console::println;

struct Xv6Syscalls {}

impl Xv6Syscalls {
    fn new() -> Xv6Syscalls {
        Xv6Syscalls{}
    }
}

impl syscalls::Xv6 for Xv6Syscalls {}

extern fn xv6_kernel_test_th() {
   loop {
        println!("xv6_kernel_test_th, tid: {}", sys_get_thread_id()); 
        sys_yield(); 
   }
}

extern fn timer_thread() {
    println!("Registering xv6 timer thread"); 
    
    loop {
         sys_recv_int(syscalls::IRQ_TIMER);
         println!("xv6: got a timer interrupt"); 
    }
}


#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            ints: Box<dyn syscalls::Interrupt + Send + Sync>,
            create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
            create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
            bdev: Box<dyn syscalls::BDev>) 
{
   
    libsyscalls::syscalls::init(s);
    libsyscalls::syscalls::init_interrupts(ints);

    println!("init xv6/core");

    println!("thread id:{}", sys_get_thread_id()); 
    
    let t = sys_create_thread("xv6_kernel_test_th", xv6_kernel_test_th); 
    //t.set_affinity(2);

    println!("Mark the thread as waiting for a bit"); 

    t.set_state(syscalls::ThreadState::Waiting); 
 
    let timer = sys_create_thread("xv6_int[timer]", timer_thread); 
    timer.set_priority(10);

   
    let (dom_xv6fs, vfs)  = create_xv6fs.create_domain_xv6fs(bdev);
    
    let xv6 = Box::new(Xv6Syscalls::new()); 

    let dom_shell  = create_xv6usr.create_domain_xv6usr("shell", xv6);

    println!("Mark the thread as runnable again"); 
    t.set_state(syscalls::ThreadState::Runnable); 

    //println!("thread:{}", t);
    drop(t); 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
