#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(asm)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(thread_local)]
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;

#[macro_use]
mod console;
mod interrupt;
mod entryother;
mod redsys;
pub mod banner;
pub mod gdt;
mod tls;

use x86::cpuid::CpuId;
use core::panic::PanicInfo;

#[no_mangle]
pub static mut others_stack: u64 = 0;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    banner::boot_banner();

    let cpu_id = 0;
    unsafe {
        gdt::init_gdt();
        let tcb_offset = tls::init_tcb(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
    }

    interrupt::init_idt();
    interrupt::init_irqs();

    interrupt::init_irqs_local();
    x86_64::instructions::interrupts::enable();

    println!("cpu0: Initialized");

    // HACK: We need to get the actual CPU topology
    let cpu1stack: [u8; 4096] = [0; 4096]; // kalloc() pls
    unsafe {
        interrupt::init_cpu(1, cpu1stack, rust_main_others as u64);
    }

    loop {
    }

    /*
    let rmr = unsafe { redsys::resources::RawMemoryRegion::<u32>::new(0xfee00000, 0x34) };
    println!("Memory region: {:?}", rmr);
    println!("Valid offset: {:?} -> 0x{:x?}", rmr.offset(0x30), *rmr.offset(0x30).unwrap());
    println!("Out-of-range: {:?}", rmr.offset(0x200));
    println!("Zero-copy slice: 0x{:x?}", rmr.as_slice()[12]);
    */

    halt();
}

#[no_mangle]
pub extern "C" fn rust_main_others() -> ! {
    let featureInfo = CpuId::new().get_feature_info()
        .expect("main_others: CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;
    unsafe {
        gdt::init_gdt();
        let tcb_offset = tls::init_tcb(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
    }

    interrupt::init_idt();

    interrupt::init_irqs_local();
    x86_64::instructions::interrupts::enable();
     
    println!("cpu{}: Initialized", cpu_id);

    halt();
}


pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
