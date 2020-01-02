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
use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_recv_int};
use console::println;

extern fn timer_thread() {
    println!("Registering timer thread"); 
    
    loop {
         sys_recv_int(syscalls::IRQ_TIMER);
         println!("init: got a timer interrupt"); 
    }
}


extern fn test_init_thread() {
   loop {
        println!("User init thread"); 
        sys_yield(); 
   }
}

extern fn test_init_thread2() {
   loop {
        println!("User init thread 2"); 
        sys_yield(); 
   }
}


// AB: XXX: The following is is not supported in Rust at the moment
//
//pub fn init(s: Box<dyn syscalls::Syscall 
//                    + syscalls::CreateXv6 + syscalls::CreateXv6FS /* + CreateXv6User */
//                    + syscalls::CreatePCI + syscalls::CreateAHCI + Send + Sync>) 
// See
//   rustc --explain E0225
//
// We have to re-write in an ugly way
#[no_mangle]
pub fn init(s: Box<dyn syscalls::Syscall + Send + Sync>,
            ints: Box<dyn syscalls::Interrupt + Send + Sync>,
            create_xv6: Box<dyn syscalls::CreateXv6>,
            create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
            create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
            create_pci: Box<dyn syscalls::CreatePCI>,
            create_ixgbe: Box<dyn syscalls::CreateIxgbe>,
            create_ahci: Box<dyn syscalls::CreateAHCI>) 
{
    libsyscalls::syscalls::init(s);

    let ints_clone = ints.int_clone(); 
    libsyscalls::syscalls::init_interrupts(ints);
    
    //let b = Box::new(4);
    //let r = sys_alloc();
    let mut v1: Vec<u64> = Vec::with_capacity(1024);
    for i in 0..2048 {
        v1.push(i);
    }

    println!("{} {} {}", "init", "userland", 1);

    //println!("init userland print works");

    let t = sys_create_thread("init_int[timer]", timer_thread); 
    t.set_priority(10);


    let start = libsyscalls::time::get_ns_time();
    println!("current time {}, waiting for 100 ms", start);

    libsyscalls::time::sys_ns_sleep(100_000_000); 

    let end = libsyscalls::time::get_ns_time();
    println!("current time {}, waited for {} ms", end, (end - start) / 1_000_000);

    /*

    let t = sys_create_thread("init_thread", test_init_thread); 
    t.set_affinity(1); 

    let t2 = sys_create_thread("init_thread_2", test_init_thread2); 
    t2.set_affinity(0); 

    drop(t); 
    drop(t2); 

    */

    let pci_resource = create_pci.get_pci_resource();

    let pci_bar = create_pci.get_pci_bar();

    let (dom_pci, pci) = create_pci.create_domain_pci(pci_resource, pci_bar);

    let pci2 = pci.pci_clone();

    let (dom_ahci, bdev) = create_ahci.create_domain_ahci(pci);

    let (dom_ixgbe, net) = create_ixgbe.create_domain_ixgbe(pci2);

    let dom_xv6 = create_xv6.create_domain_xv6kernel(ints_clone, create_xv6fs, create_xv6usr, bdev); 

}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
