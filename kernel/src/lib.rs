#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    box_syntax,
    const_fn,
    const_fn_fn_ptr_basics,
    const_mut_refs,
    const_raw_ptr_to_usize_cast,
    thread_local,
    untagged_unions,
    naked_functions,
    panic_info_message,
    asm,
    llvm_asm,
    global_asm,
    type_ascription,
)]

extern crate rust_perfcnt_bare_metal;
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate core;
extern crate spin;
#[macro_use]
extern crate alloc;
extern crate backtracer;
extern crate elfloader;
extern crate pcid;
extern crate unwind as libunwind;

use crate::interrupt::{disable_irq, enable_irq};
use core::sync::atomic::{AtomicU32, Ordering};

#[macro_use]
mod console;
mod buildinfo;
mod cb;
mod drivers;
mod dropper;
mod entryother;
pub mod gdt;
mod heap;
mod interrupt;
mod kbd;
mod redsys;
mod unwind;

mod memory;
mod multibootv2;
mod rtc;

#[macro_use]
mod prelude;
pub mod arch;

mod dev;
mod domain;
mod generated_domain_create;
mod panic;
mod pci;
mod sync;
mod syscalls;
mod thread;
mod tls;
mod waitqueue;
mod perfctr;

use crate::arch::init_buddy;
use crate::arch::memory::BASE_PAGE_SIZE;
use crate::arch::KERNEL_END;
use crate::memory::construct_pt;
use crate::multibootv2::BootInformation;
use crate::panic::{init_backtrace, init_backtrace_context};
use crate::pci::scan_pci_devs;
use core::ptr;
use rust_perfcnt_bare_metal::*;
use x86::cpuid::CpuId;
use drivers::pfc::*;

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
        let ap_cpu_stack = unsafe { crate::thread::alloc_stack() } as u64;

        println!(
            "Waking up CPU{} with stack: {:x}--{:x}",
            cpu,
            ap_cpu_stack,
            ap_cpu_stack + (crate::thread::STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE) as u64
        );

        unsafe {
            ptr::write_volatile(&mut ap_entry_running as *mut bool, true);
            interrupt::init_cpu(
                cpu,
                ap_cpu_stack + (crate::thread::STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE) as u64,
                rust_main_ap as u64,
            );
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

// [module] [kernel cpu0 stack][module]
//                             ^ KERNEL_END
//                                     ^ new KERNEL_END

pub fn init_backtrace_kernel_elf(bootinfo: &BootInformation) {
    unsafe {
        for tag in bootinfo.module_tags() {
            match tag.name() {
                "redleaf_kernel" => {
                    use alloc::vec::Vec;

                    let kelf = (tag.start_address(), tag.end_address());
                    let ksize = (kelf.1 - kelf.0) as usize;
                    println!("Found kernel image at: {:x} end : {:x}", kelf.0, kelf.1);

                    let src = kelf.0 as *const u8;
                    let dest = KERNEL_END as *mut u64 as *mut u8;
                    println!(
                        "Copying image bytes from {:x?} to {:x?} ({} bytes)",
                        src, dest, ksize
                    );

                    let mut tmpbuf: Vec<u8> = Vec::with_capacity(ksize);

                    ptr::copy(src, tmpbuf.as_mut_ptr(), ksize);
                    ptr::copy(tmpbuf.as_ptr(), dest, ksize);

                    let kernel_elf = KERNEL_END;
                    let new_end = KERNEL_END + ksize as u64;
                    KERNEL_END = round_up!(new_end, BASE_PAGE_SIZE as u64);
                    init_backtrace(core::slice::from_raw_parts(
                        kernel_elf as *const usize as *const u8,
                        ksize,
                    ));
                    elf_found = true;
                }
                _ => {
                    println!("Kernel image not found. Backtrace will be without symbols");
                }
            };
        }
    }
}

// Create sys/init domain and execute its init function
extern "C" fn init_user() {
    // die() enables interrupts as it thinks it is
    // starting a user thead, lets disable them
    disable_irq();
    generated_domain_create::create_domain_init();
    enable_irq();
}

fn start_init_thread() {
    crate::thread::create_thread("init", init_user);
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    let mut isIntel : bool = false;

    match CpuId::new().get_vendor_info() {
        Some(vendor) => {
            let vendor_string = vendor.as_string();
            println!("RedLeaf booting (CPU model: {})", vendor_string);
            if vendor_string == "GenuineIntel" {
                isIntel = true;
            }
        },
        None => println!("RedLeaf booting on (CPU model: unknown)"),
    }

    if let Some(version) = buildinfo::BUILD_VERSION {
        println!("Version: {}", version);
    }

    rtc::print_date();

    let featureInfo = CpuId::new().get_feature_info().expect("CPUID unavailable");

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

    // Init memory allocator (normal allocation should work after this)
    init_allocator(&bootinfo);

    init_backtrace_kernel_elf(&bootinfo);

    // To enable NX mappings
    unsafe {
        // Enable NXE bit (11)
        use x86::msr::{rdmsr, wrmsr, IA32_EFER, IA32_MISC_ENABLE, MSR_PLATFORM_INFO, IA32_PERF_STATUS, IA32_PERF_CTL, IA32_MPERF, IA32_APERF};
        let efer = rdmsr(IA32_EFER) | 1 << 11;
        wrmsr(IA32_EFER, efer);

        #[cfg(feature = "baremetal")]
        {
            let aperf_old = rdmsr(IA32_APERF);
            let mperf_old = rdmsr(IA32_MPERF);

            if isIntel {
                let perf_status = rdmsr(IA32_PERF_STATUS);
                let perf_ctl = rdmsr(IA32_PERF_CTL);
                println!(
                    "IA32_PERF_STATUS {:x} IA32_PERF_CTL {:x}",
                    perf_status, perf_ctl
                );

                // request 2.2GHz
                // If you want to request a different frequency, write to PERF_CTL
                //wrmsr(IA32_PERF_CTL, 0x1600);

                let mut misc = rdmsr(IA32_MISC_ENABLE);
                // Disable turbo boost
                misc |= (1u64 << 38);
                // Disable Intel speed-step technology
                misc &= !(1u64 << 16);
                wrmsr(IA32_MISC_ENABLE, misc);
                println!("IA32_MISC_ENABLE {:x}", rdmsr(IA32_MISC_ENABLE));

                // Read MSR_PLATFORM_INFO
                let plat_info = rdmsr(MSR_PLATFORM_INFO);
                println!("MSR_PLATFORM_INFO {:x}", plat_info);
                let nominal_tsc = (plat_info >> 8) & 0xff;
                let lfm = (plat_info >> 40) & 0xf;

                // FIXME: 100MHz multiplier differs with family. For c220g2/Haswell, it is 100.
                println!(
                    "Nominal TSC frequency {} MHz LFM {} MHz",
                    nominal_tsc * 100,
                    lfm * 100
                );
            }

            let aperf_delta = rdmsr(IA32_APERF) - aperf_old;
            let mperf_delta = rdmsr(IA32_MPERF) - mperf_old;
            let ratio = aperf_delta as f64 / mperf_delta as f64;
            println!("aperf/mperf ratio {}", ratio);
        }
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

    let featureInfo = CpuId::new().get_feature_info().expect("CPUID unavailable");

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
        // XXX: We already pass the new pgdir and initialize it while booting the ap_cpus
        //construct_ap_pt();
    }

    if unsafe { elf_found } {
        init_backtrace_context();
    }

    if cpu_id == 0 {
        //perfctr::list_perf_cnt();
        domain::domain::init_domains();     
        //test_perfcount(); 
        // FIXME: kbd irqhandler is broken. disable temporarily
        /*use kbd::KBDCTRL;
        use crate::drivers::Driver;
        {
            let registrar = unsafe { interrupt::get_irq_registrar(KBDCTRL.clone()) };
            KBDCTRL.lock().set_irq_registrar(registrar);
        }*/
    }

    // Init threads marking this boot thread as "idle"
    // the scheduler will treat it specially and will never schedule
    // it unless there is really no runnable threads
    // on this CPU
    //thread::init_threads();

    if cpu_id == 0 {
        // We initialized kernel domain, and the idle thread on this CPU
        // it's safe to start other CPUs, nothing will get migrated to us
        // (CPU0), but even if it will we're ready to handle it

        #[cfg(feature = "smp")]
        init_ap_cpus();

        // Create the init thread
        //
        // We add it to the scheduler queue on this CPU.
        // When we enable the interrupts below the timer interrupt will
        // kick the scheduler
        //start_init_thread();
        start_perf_count(100000,x86::perfcnt::intel::events().unwrap().get("CPU_CLK_UNHALTED.THREAD_P").unwrap()); 
    }

    enable_irq();
    for i in 0..1000{
        perf_on_backtrace();
    }
    print_perf_count_stats();
    

    
    unwind::unwind_test();

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

pub fn perf_on_backtrace(){
    use crate::panic;
    let context = match panic::ELF_CONTEXT.r#try() {
        Some(t) => t,
        None => {
            println!("ELF_CONTEXT was not initialized");
            return;
        }
    };
    let relocated_offset = panic::RELOCATED_OFFSET;
    use x86::current::registers;
    backtracer::resolve(context.as_ref(), relocated_offset,registers::rip() as *mut u8, |symbol| {
        match symbol.name() {
            Some(fun_name) => {
            },
            None => {
            },
        }
               });
}