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

#[macro_use]
extern crate bitflags;
extern crate byteorder;
#[macro_use]
extern crate serde_derive;

extern crate malloc;
extern crate alloc;

mod bar;
mod bus;
mod class;
mod dev;
mod func;
mod header;
mod pci;

use core::panic::PanicInfo;
use syscalls::syscalls::{Syscall};
use libsyscalls::syscalls::{sys_print, sys_alloc};
use console::println;

#[no_mangle]
pub fn pci_init(s: Syscall) {
    libsyscalls::syscalls::init(s);

    sys_print("init PCI domain");


}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
