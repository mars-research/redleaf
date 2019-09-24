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
    untagged_unions
)]

extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;
extern crate slabmalloc;
#[macro_use]
extern crate alloc;
extern crate pcid;

#[macro_use]
mod console;
mod interrupt;
mod entryother;
mod redsys;
pub mod gdt;


mod multibootv2;
mod memory;
mod prelude;
pub mod arch;

mod tls;
mod pci;

use x86::cpuid::CpuId;
use core::panic::PanicInfo;
use crate::arch::init_buddy;
use crate::memory::construct_pt;
use crate::pci::scan_pci_devs;

#[no_mangle]
pub static mut cpu1_stack: u32 = 0;

extern "C" {
    #[no_mangle]
    static _bootinfo: usize;
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

// Init AP cpus
pub fn init_ap_cpus() {

    // Allocate CPU stack

    // HACK: We need to get the actual CPU topology
    unsafe {
        interrupt::init_cpu(1, cpu1_stack, rust_main_ap as u64);
    }
}

pub fn init_allocator() {
    unsafe {
        println!("multibootv2 tag found at {:x}", _bootinfo as usize);
        let bootinfo = multibootv2::load(_bootinfo);
        println!("Tags: {:?}", bootinfo);
        init_buddy(bootinfo);
    }
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

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;

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
    interrupt::init_irqs();

    // Microkernel runs with interrupts disabled
    // we re-enable them on exits
    //x86_64::instructions::interrupts::enable();
     
    // Spin up other CPUs 
    init_ap_cpus(); 

    rust_main_ap(); 
}

#[no_mangle]
pub extern "C" fn rust_main_ap() -> ! {
    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;
    println!("Initializing CPU#{}", cpu_id); 

    unsafe {
        gdt::init_global_gdt();
        let tcb_offset = tls::init_per_cpu_vars(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
    }

    interrupt::init_idt();

    interrupt::init_irqs_local();
    //x86_64::instructions::interrupts::enable();
     
    println!("cpu{}: Initialized", cpu_id);

    halt(); 
}


pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
