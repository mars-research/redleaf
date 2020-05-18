#![no_std]
#![forbid(unsafe_code)]
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

#[macro_use]
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::panic::PanicInfo;

use syscalls::{Syscall, Heap};
use usrlib::{println, print};
use usrlib::syscalls::{sys_open, sys_fstat, sys_read, sys_write, sys_close};
use usr::xv6::Xv6;
use usr::vfs::{VFSPtr, DirectoryEntry, DirectoryEntryRef, INodeFileType, FileMode};

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, rv6: Box<dyn Xv6>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone());
    println!("Starting rv6 benchnet with args: {}", args);

    libbenchnet::run_tx_udptest_rref(rv6.as_net(), 64, false);
    // libbenchnet::run_rx_udptest_rref(&mut rv6, 64, false);
    // libbenchnet::run_fwd_udptest_rref(&mut rv6, 64);
}


// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchnet panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
