#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    const_raw_ptr_to_usize_cast,
    thread_local,
    untagged_unions
)]

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

mod multibootv2;
mod memory;
mod prelude;
pub mod arch;

mod tls;

use x86::cpuid::CpuId;
use core::panic::PanicInfo;

#[no_mangle]
pub static mut cpu1_stack: u32 = 0;

extern "C" {
    #[no_mangle]
    static _bootinfo: usize;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    halt();
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    let featureInfo = CpuId::new().get_feature_info()
        .expect("CPUID unavailable");

    let cpu_id: u32 = featureInfo.initial_local_apic_id() as u32;
    unsafe {
        gdt::init_gdt();
        let tcb_offset = tls::init_tcb(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
    }

    interrupt::init_idt();

    if cpu_id == 0 {
        // Initialize LAPIC as BSP
        banner::boot_banner();
        unsafe {
            println!("multibootv2 tag found at {:x}", _bootinfo as usize);
            println!("Tags: {:?}", multibootv2::load(_bootinfo));
        }
        interrupt::init_irqs();
    }

    interrupt::init_irqs_local();
    x86_64::instructions::interrupts::enable();
     
    println!("cpu{}: Initialized", cpu_id);

    if cpu_id == 0 {
        // Spin up other CPUs as BSP

        // HACK: We need to get the actual CPU topology
        unsafe {
            interrupt::init_cpu(1, cpu1_stack, rust_main as u64);
        }
    }

    loop {}
}


pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
