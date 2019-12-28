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
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::println;
//use pci::Pci;

/*
 * fn get_ixgbe_traitobj() -> impl IxgbeBarRegion { }
 */

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>) {

}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
