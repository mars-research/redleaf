#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
    maybe_uninit_extra
)]

extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::panic::PanicInfo;
use syscalls::{Heap, Syscall};

use console::println;
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;
use interface::rpc::RpcResult;

pub use interface::error::{ErrorKind, Result};

use rref::RRefDeque;



#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    pci: Box<dyn interface::pci::PCI>,
) -> Box<dyn interface::net::Net> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    unimplemented!()
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
