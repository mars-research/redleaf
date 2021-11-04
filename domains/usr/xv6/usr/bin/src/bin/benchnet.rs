#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(untagged_unions)]

extern crate alloc;
extern crate malloc;

#[macro_use]
use alloc::boxed::Box;

use core::panic::PanicInfo;

use syscalls::{Heap, Syscall};

use interface::rv6::Rv6;

use usrlib::println;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Rv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone_rv6().unwrap());
    println!("Starting rv6 benchnet with args: {}", args);

    let net = rv6.as_net().unwrap();

    libbenchnet::run_domain_crossing(&*net);

    for _ in 0..5 {
        libbenchnet::run_tx_udptest_rref(&*net, 64, false);
    }

    /*for _ in 0..5 {
        libbenchnet::run_fwd_udptest_rref(&*net, 64);
    }*/

    /*
    for _ in 0..5 {
        libbenchnet::run_tx_udptest_rref(&*net, 1514, false);
    }*/

    /*for d in (0..=1000).step_by(100) {
        libbenchnet::run_rx_udptest_rref_with_delay(&*net, 64, false, d);
    }*/

    /*for _ in 0..5 {
        libbenchnet::run_rx_udptest_rref(&*net, 64, false);
    }*/

    panic!("");

    libbenchnet::run_tx_udptest_rref(&*net, 64, false);
    libbenchnet::run_fwd_udptest_rref(&*net, 64);
    libbenchnet::run_maglev_fwd_udptest_rref(&*net, 64);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchnet panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
