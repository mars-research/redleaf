#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_backtrace};
use console::println;
use protocol::UdpPacket;
use spin::Mutex;
use alloc::sync::Arc;

fn construct_udp_packet() -> Arc<Mutex<UdpPacket>> {
    Arc::new(Mutex::new(UdpPacket::new_zeroed()))
}

#[no_mangle]
pub fn kvstore_init(s: Box<dyn syscalls::Syscall + Send + Sync>, net: Box<dyn syscalls::Net>)
{
    libsyscalls::syscalls::init(s);
    let buf = [0, 1, 2];
    net.send(&buf);
    net.send_udp(construct_udp_packet());
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_backtrace();
    loop {}
}
