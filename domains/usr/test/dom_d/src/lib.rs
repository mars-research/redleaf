#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use syscalls::{Heap, Syscall};

use alloc::boxed::Box;

use console::println;

use core::panic::PanicInfo;

use interface::rref::RRef;

pub fn main(dom_c: Box<dyn interface::dom_c::DomC>) {
    println!("Init domain D");

    let iter = 10_000_000;

    let start = libtime::get_rdtsc();
    for _ in 0..iter {
        dom_c.no_arg().unwrap();
    }
    let elapse = libtime::get_rdtsc() - start;
    println!(
        "dom_c.no_arg: avg: {}, total: {}, iter: {}",
        elapse as f64 / iter as f64,
        elapse,
        iter
    );

    let start = libtime::get_rdtsc();
    for _ in 0..iter {
        dom_c.one_arg(1).unwrap();
    }
    let elapse = libtime::get_rdtsc() - start;
    println!(
        "dom_c.one_arg: avg: {}, total: {}, iter: {}",
        elapse as f64 / iter as f64,
        elapse,
        iter
    );
    assert!(dom_c.one_arg(12321).unwrap() == 12321 + 1);

    let start = libtime::get_rdtsc();
    let mut x = RRef::new(0usize);
    for _ in 0..iter {
        x = dom_c.one_rref(x).unwrap();
    }
    let elapse = libtime::get_rdtsc() - start;
    println!(
        "dom_c.one_rref: avg: {}, total: {}, iter: {}",
        elapse as f64 / iter as f64,
        elapse,
        iter
    );
    assert!(*dom_c.one_rref(x).unwrap() == iter + 1);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
