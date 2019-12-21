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
use libsyscalls::syscalls::{sys_print, sys_alloc, sys_create_thread};
use console::println;
//use pci::Pci;


// Implementation not visible to the ixgbe domain

/*fn get_ixgbe() -> impl IxgbeBarRegion {
    let ixgbe_bar = IxgbeBar::new(0x8_0000, 0x1000);
    ixgbe_bar
}*/

#[no_mangle]
pub fn init(s: Syscall) {

}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
