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
use crate::interrupt::{disable_irq, enable_irq};
use core::sync::atomic::{AtomicU32, Ordering};

#[macro_use]
mod console;
mod interrupt;
mod entryother;
mod redsys;
mod drivers;
mod heap;
mod buildinfo;
pub mod gdt;


mod multibootv2;
mod memory;
mod rtc;

#[macro_use]
mod prelude;
pub mod arch;

mod tls;
mod thread;
mod panic;
mod syscalls;
mod pci;
mod domain;
mod dev;
mod waitqueue;

use x86::cpuid::CpuId;
use crate::arch::{init_buddy};
use crate::memory::{construct_pt, construct_ap_pt};
use crate::pci::scan_pci_devs;
use core::ptr;
use crate::arch::memory::BASE_PAGE_SIZE;
use crate::arch::{KERNEL_END, kernel_end};
use crate::panic::{init_backtrace, init_backtrace_context};
use crate::multibootv2::BootInformation;

pub static mut ap_entry_running: bool = true;
pub const MAX_CPUS: u32 = 4;
static RUNNING_CPUS: AtomicU32 = AtomicU32::new(0);

static mut elf_found: bool = false;

extern "C" {
    #[no_mangle]
    static _bootinfo: usize;
}

pub fn active_cpus() -> u32 {
    RUNNING_CPUS.load(Ordering::Relaxed)
}


// Note, the bootstrap CPU runs on a statically allocated
// stack that is defined in boot.asm
// AB TODO: fix this (i.e., switch to the dynamically allocated stack)

// Init AP cpus
pub fn init_ap_cpus() {
    for cpu in 1..MAX_CPUS {
        let ap_cpu_stack = unsafe { crate::thread::alloc_stack() } as u32;

        println!("Waking up CPU{} with stack: {:x}--{:x}",
            cpu, ap_cpu_stack, 
            ap_cpu_stack + (crate::thread::STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE) as u32);

        unsafe {
            ptr::write_volatile(&mut ap_entry_running as *mut bool, true);
            interrupt::init_cpu(cpu,
                ap_cpu_stack + (crate::thread::STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE) as u32,
                rust_main_ap as u64);
        }

        while unsafe { ptr::read_volatile(&ap_entry_running as *const bool) } {}
    }

    while RUNNING_CPUS.load(Ordering::SeqCst) != (MAX_CPUS - 1) {
        // We can't halt here, interrupts are still off
    }

    println!("Done initializing APs");
}

pub fn init_allocator(bootinfo: &BootInformation) {
    unsafe {
        match bootinfo.command_line_tag() {
            None => println!("No kernel command line specified"),
            Some(cmdline) => println!("Command Line: {}", cmdline.cmdline()),
        }

        println!("Tags: {:?}", bootinfo);
        init_buddy(bootinfo);
    }
}

pub fn init_backtrace_kernel_elf(bootinfo: &BootInformation) {
    unsafe {
        for tag in bootinfo.module_tags() {
            match tag.name() {
                "redleaf_kernel" => {
                    let kelf = (tag.start_address(), tag.end_address());
                    let ksize = (kelf.1 - kelf.0) as usize;
                    println!("Found kernel image at: {:x} end : {:x}", kelf.0, kelf.1);
                    ptr::copy(kelf.0 as *const u8, KERNEL_END as *mut u64 as *mut u8, ksize);

                    let kernel_elf = KERNEL_END;
                    let new_end = KERNEL_END + ksize as u64;
                    KERNEL_END = round_up!(new_end, BASE_PAGE_SIZE as u64);
                    println!("Old kernel_end: {:x} New kernel_end: {:x}", kernel_end(), new_end);
                    init_backtrace(core::slice::from_raw_parts(kernel_elf as *const usize as *const u8, ksize));
                    elf_found = true;
                },
                _ => {
                    println!("Kernel image not found. Backtrace will be without symbols");
                }
            };
        }
    }
}

// Create sys/init domain and execute its init function
extern fn init_user() {
    // die() enables interrupts as it thinks it is
    // starting a user thead, lets disable them
    disable_irq();
    crate::domain::create_domain::create_domain_init();
    enable_irq();
}

fn start_init_thread() {
    crate::thread::create_thread("init", init_user);
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {

    match CpuId::new().get_vendor_info() {
        Some(vendor) => println!("RedLeaf booting (CPU model: {})", vendor.as_string()),
        None => println!("RedLeaf booting on (CPU model: unknown)"),
    }

    println!("Version: {}", buildinfo::BUILD_VERSION);

    rtc::print_date();

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

    let bootinfo = unsafe {
        println!("multibootv2 tag found at {:x}", _bootinfo as usize);
        multibootv2::load(_bootinfo)
    };

    init_backtrace_kernel_elf(&bootinfo);

    // Init memory allocator (normal allocation should work after this)
    init_allocator(&bootinfo);

    // To enable NX mappings
    unsafe {
        // Enable NXE bit (11)
        use x86::msr::{rdmsr, wrmsr, IA32_EFER};
        let efer = rdmsr(IA32_EFER) | 1 << 11;
        wrmsr(IA32_EFER, efer);
    }

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
        ptr::write_volatile(&mut ap_entry_running as *mut bool, false);
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
        gdt::init_percpu_gdt(tcb_offset as u64);

        // Update cpuid of this CPU
        tls::set_cpuid(cpu_id as usize);

    }

    if cpu_id != 0 {
        interrupt::init_idt();
        interrupt::init_irqs_local();

        // Init page table (code runs on a new page table after this call)
        construct_ap_pt();
    }

    if unsafe { elf_found } {
        init_backtrace_context();
    }

    if cpu_id == 0 {
        domain::domain::init_domains();

    }

    // Init threads marking this boot thread as "idle" 
    // the scheduler will treat it specially and will never schedule 
    // it unless there is really no runnable threads
    // on this CPU
    thread::init_threads(); 

    if cpu_id == 0 {
        // We initialized kernel domain, and the idle thread on this CPU
        // it's safe to start other CPUs, nothing will get migrated to us 
        // (CPU0), but even if it will we're ready to handle it

        #[cfg(feature="smp")]
        init_ap_cpus();

        // Create the init thread
        //
        // We add it to the scheduler queue on this CPU. 
        // When we enable the interrupts below the timer interrupt will 
        // kick the scheduler
        start_init_thread(); 
    }

    println!("cpu{}: Initialized", cpu_id);
    println!("cpu{}: Ready to enable interrupts", cpu_id);

    RUNNING_CPUS.fetch_add(1, Ordering::SeqCst);

    // Enable interrupts; the timer interrupt will schedule the next thread
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
