#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(asm)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(thread_local)]
#![feature(naked_functions)]
#![feature(const_fn)]
extern crate x86;
#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate core;

#[macro_use]
mod console;
mod interrupt;
mod entryother;
mod banner;
mod gdt;
mod tls;
//mod common; 
mod thread;

use core::panic::PanicInfo;
use thread::{Scheduler, Thread};

#[no_mangle]
pub static mut others_stack: u64 = 0;

static cpu1stack: [u8; 4096] = [0; 4096];

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
    x86_64::instructions::interrupts::enable();

    // invoke a breakpoint exception
    // x86_64::instructions::interrupts::int3(); 

    println!("boot ok");

    let mut s = Scheduler::new();
    let mut idle = Thread::new("idle");
    let mut t1 = Thread::new("hello 1");
    let mut t2 = Thread::new("hello 2");

    let mut idle = Thread::new("idle");

    s.put_thread(&mut t1);
    s.put_thread(&mut t2);


    // HACK: We need to get the actual CPU topology
    unsafe {
        interrupt::init_cpu(1, cpu1stack, rust_main_others as u64);
    }

    halt();
}

#[no_mangle]
pub extern "C" fn rust_main_others() -> ! {

    let cpu_id = 1;
    unsafe {
        gdt::init_gdt();
        let tcb_offset = tls::init_tcb(cpu_id);
        gdt::init_percpu_gdt(tcb_offset);
    }

    interrupt::init_idt();

    // interrupt::init_irqs();
    // x86_64::instructions::interrupts::enable();

    // invoke a breakpoint exception
    // x86_64::instructions::interrupts::int3(); 
     
    println!("booted another CPU ok");

    halt();
}


pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
