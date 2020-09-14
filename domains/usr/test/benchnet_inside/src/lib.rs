#![no_std]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::{println, print};
use core::alloc::Layout;
use core::panic::PanicInfo;
use usr;
use alloc::vec::Vec;
use usr::net::Net;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, net: Box<dyn Net>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init domain benchnet_inside");

    /*
    for _ in 0..5 {
        libbenchnet::run_tx_udptest_rref(&*net, 64, false);
    }
    */

    //panic!("");
    /*for _ in 0..5 {
        libbenchnet::run_tx_udptest_rref(&*net, 1514, false);
    }*/
    /*for d in (0..=1000).step_by(100) {
        libbenchnet::run_fwd_udptest_rref_with_delay(&*net, 64, d);
    }*/

    /*for _ in 0..5 {
        libbenchnet::run_fwd_udptest_rref(&*net, 64);
    }*/

    /*for _ in 0..5 {
        // for d in (0..=1000).step_by(100) {
        libbenchnet::run_rx_udptest_rref_with_delay(&*net, 64, false, 0);
    }*/

    /*for _ in 0..5 {
        libbenchnet::run_tx_udptest_rref(&*net, 1514, false);
    }*/

    /*
    libbenchnet::run_tx_udptest_rref(&*net, 64, false);
    libbenchnet::run_rx_udptest_rref(&*net, 64, false);
    libbenchnet::run_fwd_udptest_rref(&*net, 64);
    */
    libbenchnet::run_maglev_fwd_udptest_rref(&*net, 64);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
