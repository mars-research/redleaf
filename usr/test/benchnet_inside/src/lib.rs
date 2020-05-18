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
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, mut net: Box<dyn Net>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init domain benchnet_inside");

    libbenchnet::run_tx_udptest_rref(&mut net, 64, false);
    libbenchnet::run_rx_udptest_rref(&mut net, 64, false);
    libbenchnet::run_fwd_udptest_rref(&mut net, 64);
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
