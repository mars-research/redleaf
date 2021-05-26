#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra
)]

extern crate alloc;
extern crate malloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{boxed::Box, collections::BTreeMap};
use core::{borrow::BorrowMut, panic::PanicInfo, pin::Pin, usize};
use syscalls::{Heap, Syscall};

use console::{print, println};
use interface::bdev::BSIZE;
use interface::{net::Net, rpc::RpcResult};
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;
use virtio_block_device::pci::PciFactory;

pub use interface::error::{ErrorKind, Result};

pub struct VirtioBlock(Arc<Mutex<VirtioBlockInner>>);

use interface::rref::{RRef, RRefDeque};

impl interface::bdev::BDev for VirtioBlock {
    fn read(&self, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>> {
        unimplemented!();
    }
    fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()> {
        unimplemented!();
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    pci: Box<dyn interface::pci::PCI>,
) -> Box<dyn interface::bdev::BDev> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    #[cfg(feature = "virtio_block")]
    println!("Virtio Block starting");

    let blk = {
        let blk = {
            let mut pci_factory = PciFactory::new();
            if pci.pci_register_driver(&mut pci_factory, 4, None).is_err() {
                panic!("Failed to probe VirtioBlock PCI");
            }
            let dev = pci_factory.to_device().unwrap();
            VirtioBlock(Arc::new(Mutex::new(dev)))
        };
        blk.0.lock().init();
        blk
    };

    Box::new(blk)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
