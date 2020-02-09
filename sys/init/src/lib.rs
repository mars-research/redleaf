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
    panic_info_message,
    get_mut_unchecked
)]

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_recv_int, sys_backtrace};
use console::println;

#[cfg(feature = "test_guard_page")]
fn test_stack_exhaustion() -> u64 {
   
    let mut t: [u64; 4096] = [0; 4096];
    t[0] = t[1] + test_stack_exhaustion();
    t[0]
}


#[cfg(feature = "test_timer_thread")]
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

#[cfg(feature = "test_sleep")]
fn test_sleep() {
    let start = libtime::get_ns_time();
    println!("current time {}, waiting for 10_000 ms", start);

    libtime::sys_ns_sleep(10_000_000_000); 

    let end = libtime::get_ns_time();
    println!("current time {}, waited for {} ms", end, (end - start) / 1_000_000);

}

fn test_dummy_syscall() {
    use libsyscalls::syscalls::sys_dummy;
    use libtime::get_rdtsc;

    let NUM_ITER: u64 = 20_000_000;
    let start = get_rdtsc();

    for i in 0..NUM_ITER {
        sys_dummy();
    }

    let elapsed = get_rdtsc() - start;
    println!("Dummy syscall test: {} iterations took {} (avg: {} cycles)", NUM_ITER,
                        elapsed, elapsed / NUM_ITER);
}

// AB: XXX: The following is is not supported in Rust at the moment
//
//pub fn init(s: Box<dyn syscalls::Syscall 
//                    + create::CreateXv6 + create::CreateXv6FS /* + CreateXv6User */
//                    + create::CreatePCI + create::CreateAHCI + Send + Sync>) 
// See
//   rustc --explain E0225
//
// We have to re-write in an ugly way
#[no_mangle]
pub fn init(s: Box<dyn syscalls::Syscall + Send + Sync>,
            ints: Box<dyn syscalls::Interrupt + Send + Sync>,
            heap: Box<dyn syscalls::Heap + Send + Sync>,
            create_proxy: Box<dyn create::CreateProxy>,
            create_xv6: Box<dyn create::CreateXv6>,
            create_xv6fs: Box<dyn create::CreateXv6FS>,
            create_xv6usr: Box<dyn create::CreateXv6Usr>,
            create_pci: Box<dyn create::CreatePCI>,
            create_ixgbe: Box<dyn create::CreateIxgbe>,
            create_ahci: Box<dyn create::CreateAHCI>) 
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


    #[cfg(feature = "test_guard_page")]
    {
        println!("start the test guard page test"); 
        let foo =  test_stack_exhaustion();
        println!("test guard page: {}", foo); 
    }

    #[cfg(feature = "test_timer_thread")]
    {
        let t = sys_create_thread("init_int[timer]", timer_thread); 
        t.set_priority(10);
    }

    #[cfg(feature = "test_sleep")]
    test_sleep();

    
    #[cfg(feature = "test_threads")]
    {
        let t = sys_create_thread("init_thread", test_init_thread); 
        t.set_affinity(1); 

        let t2 = sys_create_thread("init_thread_2", test_init_thread2); 
        t2.set_affinity(0); 

        #[cfg(feature = "test_sleep")]
        test_sleep();

        println!("Setting affinity to CPUs 2 and 3"); 
        t.set_affinity(2); 
        t2.set_affinity(3); 

         #[cfg(feature = "test_sleep")]
        test_sleep();

        println!("Setting affinity to CPUs 1 and 1"); 
        t.set_affinity(1); 
        t2.set_affinity(1); 

        drop(t); 
        drop(t2); 
    }

    let mut proxy_bdev: Arc<(Option<u64>, Option<Box<dyn usr::bdev::BDev>>)> = Arc::new((None, None));
    // test_dummy_syscall();

    println!("about to create proxy");
    let (dom_proxy, proxy) = create_proxy.create_domain_proxy(heap, proxy_bdev.clone());
    println!("created proxy");

    let pci_resource = create_pci.get_pci_resource();

    let pci_bar = create_pci.get_pci_bar();

    let (dom_pci, pci) = create_pci.create_domain_pci(pci_resource, pci_bar);

    let pci2 = pci.pci_clone();

    let (dom_ahci, bdev) = create_ahci.create_domain_ahci(pci);

    // TODO: threadsafe?
    unsafe {
        let mut proxy_bdev = Arc::get_mut_unchecked(&mut proxy_bdev);
        proxy_bdev.0.replace(dom_ahci.get_domain_id());
        proxy_bdev.1.replace(bdev);
    }

    let (dom_ixgbe, net) = create_ixgbe.create_domain_ixgbe(pci2);

    let dom_xv6 = create_xv6.create_domain_xv6kernel(ints_clone, create_xv6fs, create_xv6usr, proxy.proxy_clone());
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_backtrace();
    loop {}
}
