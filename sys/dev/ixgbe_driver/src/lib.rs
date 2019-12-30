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
use syscalls::{Syscall,PCI};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::println;

struct Ixgbe {}

impl Ixgbe {
    fn new() -> Ixgbe {
        Ixgbe{}
    }
}

impl syscalls::Net for Ixgbe {
}

#[no_mangle]
pub fn ixgbe_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net> {
    libsyscalls::syscalls::init(s);

    println!("ixgbe_init: starting ixgbe driver domain");
    Box::new(Ixgbe::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
