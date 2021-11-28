#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use console::println;
use ixgbe;
use syscalls::{Heap, Syscall};

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    pci: Box<dyn interface::pci::PCI>,
) -> Box<dyn interface::net::Net> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("ixgbe_trusted_entry_new!!!");

    ixgbe::main(pci)
}
