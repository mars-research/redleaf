#![no_std]
#![no_main]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};

use alloc::boxed::Box;

use console::println;

use core::panic::PanicInfo;
use usr;
use rref::{RRef};


#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, dom_c: Box<dyn usr::dom_c::DomC>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init domain D");

    let iter = 10_000_000;

    let start = libtime::get_rdtsc();
    for _ in 0..iter {
        dom_c.no_arg().unwrap();
    }
    let elapse = libtime::get_rdtsc() - start;
    println!("dom_c.no_arg: avg: {}, total: {}, iter: {}", elapse as f64 / iter as f64, elapse, iter);

    let start = libtime::get_rdtsc();
    for _ in 0..iter {
        dom_c.one_arg(1).unwrap();
    }
    let elapse = libtime::get_rdtsc() - start;
    println!("dom_c.one_arg: avg: {}, total: {}, iter: {}", elapse as f64 / iter as f64, elapse, iter);
    assert!(dom_c.one_arg(12321).unwrap() == 12321 + 1);

    let start = libtime::get_rdtsc();
    let mut x = RRef::new(0usize);
    for _ in 0..iter {
        x = dom_c.one_rref(x).unwrap();
    }
    let elapse = libtime::get_rdtsc() - start;
    println!("dom_c.one_rref: avg: {}, total: {}, iter: {}", elapse as f64 / iter as f64, elapse, iter);
    assert!(*dom_c.one_rref(x).unwrap() == iter + 1);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
