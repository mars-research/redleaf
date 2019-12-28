#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    thread_local,
    untagged_unions,
    naked_functions,
    panic_info_message,
    param_attrs
)]

extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;
extern crate slabmalloc;
#[macro_use]
extern crate alloc;
extern crate backtracer;
extern crate pcid;
extern crate elfloader;

#[macro_use]
mod console;
mod interrupt;
mod entryother;
mod redsys;
mod drivers;
pub mod gdt;


mod multibootv2;
mod memory;
mod prelude;
pub mod arch;

mod tls;
mod thread;
mod panic; 
mod syscalls;
mod pci;
mod domain;
mod dev;

use x86::cpuid::CpuId;
use crate::arch::init_buddy;
use core::alloc::{Layout};
use crate::interrupt::{enable_irq};
use crate::memory::{construct_pt, construct_ap_pt};
use crate::pci::scan_pci_devs;

pub static mut ap_entry_running: bool = true;

extern "C" {
    #[no_mangle]
    static _bootinfo: usize;
}

/// Stack size for the kernel main thread
// Note, the bootstrap CPU runs on a statically allocated 
// stack that is defined in boot.asm 
// AB TODO: fix this (i.e., switch to the dynamically allocated stack)
const KERNEL_STACK_SIZE: usize = 4096 * 64;

// Init AP cpus
pub fn init_ap_cpus() {
    for cpu in (1..4) {
        let ap_cpu_stack = unsafe { alloc::alloc::alloc(
                    Layout::from_size_align_unchecked(KERNEL_STACK_SIZE, 4096)) } as u32;
    
        println!("Waking up CPU{} with stack: {:x}--{:x}",
            cpu, ap_cpu_stack, ap_cpu_stack + KERNEL_STACK_SIZE as u32);

        unsafe {
            ap_entry_running = true;
            interrupt::init_cpu(cpu, ap_cpu_stack + KERNEL_STACK_SIZE as u32, rust_main_ap as u64);
        }

        while unsafe { ap_entry_running } {}
    }

    println!("Done initializing APs");
}

pub fn init_allocator() {
    unsafe {
        println!("multibootv2 tag found at {:x}", _bootinfo as usize);
        let bootinfo = multibootv2::load(_bootinfo);
        println!("Tags: {:?}", bootinfo);
        init_buddy(bootinfo);
    }
}

pub extern fn hello1() {
    loop {
        println!("hello 1"); 
    }
}

pub extern fn hello2() {
    loop {
        println!("hello 2"); 
    }
}

fn test_threads() {
    crate::thread::create_thread("hello 1", hello1); 
    crate::thread::create_thread("hello 2", hello2); 
}


fn init_user() {
    crate::domain::create_domain::create_domain_init(); 
}

const MAX_CPUS: u32 = 32;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {

    match CpuId::new().get_vendor_info() {
        Some(vendor) => println!("RedLeaf booting (CPU model: {})", vendor.as_string()),
        None => println!("RedLeaf booting on (CPU model: unknown)"),
    }
    
    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    //let cpu_id: u32 = 
    featureInfo.initial_local_apic_id() as u32;

    unsafe {
        // We don't have per-CPU variables yet, init global gdt
        gdt::init_global_gdt();
    }

    // Init IDT mostly so if we get some exceptions in the allocator 
    // we can see nice crash reports
    interrupt::init_idt();

    // Init memory allocator (normal allocation should work after this) 
    init_allocator();

    // Init page table (code runs on a new page table after this call)
    construct_pt();

    scan_pci_devs();

    // Init per-CPU variables
    unsafe {
        tls::init_per_cpu_area(MAX_CPUS);
    }

    // Initialize LAPIC as BSP
    interrupt::init_irqs_local();
    interrupt::init_irqs();

    // Microkernel runs with interrupts disabled
    // we re-enable them on exits
    //x86_64::instructions::interrupts::enable();
     
    //panic!("Test panic in main()"); 
    rust_main_ap(); 
}

#[no_mangle]
pub extern "C" fn rust_main_ap() -> ! {
    unsafe {
        ap_entry_running = false;
    }

    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;
    println!("Initializing CPU#{}", cpu_id); 

    unsafe {
        if cpu_id != 0 {
            gdt::init_global_gdt();
        }

        let tcb_offset = tls::init_per_cpu_vars(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
        
        // Update cpuid of this CPU
        tls::set_cpuid(cpu_id as usize); 

    }

    if cpu_id != 0 {
        interrupt::init_idt();
        interrupt::init_irqs_local();

        // Init page table (code runs on a new page table after this call)
        construct_ap_pt();
    }
    
    if cpu_id == 0 {
        domain::domain::init_domains(); 

        // We initialized kernel domain, it's safe to start 
        // other CPUs 
        //init_ap_cpus(); 
    }

    println!("cpu{}: Initialized", cpu_id);
    thread::init_threads(); 
   
    /*
    // Initialize hello driver
    if cpu_id == 0 {
        use drivers::hello::Hello;

        println!("Initializing hello driver");
        let driver = Arc::new(Mutex::new(Hello::new()));

        {
            let registrar = unsafe { interrupt::get_irq_registrar(driver.clone()) };
            driver.lock().set_irq_registrar(registrar);
        }
    }

    // Initialize IDE driver
    if cpu_id == 0 {
        use drivers::ide::IDE;

        println!("Initializing IDE");

        let ataPioDevice = unsafe { Arc::new(Mutex::new(redsys::devices::ATAPIODevice::primary())) };
        let driver = Arc::new(Mutex::new(IDE::new(ataPioDevice, false)));

        {
            let registrar = unsafe { interrupt::get_irq_registrar(driver.clone()) };
            driver.lock().set_irq_registrar(registrar);
            driver.lock().init();
        }

        println!("IDE Initialized!");

        println!("Writing");
        // Write a block of 5s
        let data: [u32; 512] = [5u32; 512];
        driver.lock().write(20, &data);
        println!("Data written");

        // Read the block back
        let mut rdata: [u32; 512] = [0u32; 512];
        driver.lock().read(20, &mut rdata);
        println!("First byte read is {}", data[0]);
        println!("Data read");
    }

    */
   
    println!("cpu{}: Ready to enable interrupts", cpu_id);

    if cpu_id == 0 {
        //test_threads(); 

        // The first user system call will re-enable interrupts on 
        // exit to user
        init_user();
    }

    // Enable interrupts and the timer will schedule the next thread
    enable_irq();


    halt(); 
}


pub fn halt() -> ! {
    loop {
        //x86_64::instructions::interrupts::enable();
        //println!(".");
        x86_64::instructions::hlt();
    }
}
