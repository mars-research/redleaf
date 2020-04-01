#![no_std]
#![forbid(unsafe_code)]
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
use alloc::sync::Arc;
use core::panic::PanicInfo;
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_current_thread, sys_yield, sys_recv_int};
use usr::bdev::BDev;

use console::println;


struct Xv6Syscalls {}

impl Xv6Syscalls {
    fn new() -> Xv6Syscalls {
        Xv6Syscalls{}
    }
}

impl usr::xv6::Xv6 for Xv6Syscalls {}

extern fn xv6_kernel_test_th() {
   loop {
        println!("xv6_kernel_test_th, tid: {}", sys_current_thread().get_id());
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

#[cfg(feature = "test_threads")]
fn test_threads() {
    let t = sys_create_thread("xv6_kernel_test_th", xv6_kernel_test_th); 
    t.set_affinity(2);

    println!("Mark the thread as waiting for a bit"); 

    t.set_state(syscalls::ThreadState::Waiting); 
 
    let timer = sys_create_thread("xv6_int[timer]", timer_thread); 
    timer.set_priority(10);

    println!("Mark the thread as runnable again"); 
    t.set_state(syscalls::ThreadState::Runnable); 

}

#[cfg(feature = "test_sleeplock")]
fn test_sleeplock() {
    const ONE_MS_IN_NS: u64 = 1_000_000;

    let sleeplock = SleepLock::new();

    let threads = (0..3)
        .map({ |i|
            sys_create_thread("xv6_kernel_test_sleeplock", {
                loop {
                    println!("[{}]: about to acquire sleeplock", i);
                    sleeplock.acquire();
                    println!("[{}]: start of my turn", i);
                    sys_ns_sleep(ONE_MS_IN_NS * 100);
                    println!("[{}]: end of my turn", i);
                    sleeplock.release();
                    println!("[{}]: released sleeplock", i);
                }
            })
        });

    sys_ns_sleep(ONE_MS_IN_NS * 5_000);

    for thread in threads {
        thread.set_state(syscalls::ThreadState::Waiting);
    }
}

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            ints: Box<dyn syscalls::Interrupt + Send + Sync>,
            create_xv6fs: &dyn create::CreateXv6FS,
            create_xv6usr: &dyn create::CreateXv6Usr,
            bdev: Box<dyn BDev + Send + Sync>)
{
   
    libsyscalls::syscalls::init(s);
    libsyscalls::syscalls::init_interrupts(ints);

    println!("init xv6/core");

    println!("thread id:{}", sys_current_thread().get_id());


    #[cfg(feature = "test_threads")]
    test_threads();

    #[cfg(feature = "test_sleeplock")]
    test_sleeplock();

    let (_dom_xv6fs, _vfs)  = create_xv6fs.create_domain_xv6fs(bdev);

    let xv6 = Box::new(Xv6Syscalls::new());

    let _dom_shell  = create_xv6usr.create_domain_xv6usr("shell", xv6);

}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
