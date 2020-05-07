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
    panic_info_message,
    maybe_uninit_extra,
    core_intrinsics,
)]
#![forbid(unsafe_code)]

extern crate malloc;
extern crate alloc;

mod device;

use alloc::collections::VecDeque;
use alloc::boxed::Box;
#[macro_use]
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall, PCI, Heap};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::{println, print};
use pci_driver::DeviceBarRegions;
pub use libsyscalls::errors::Result;
use core::cell::RefCell;
use alloc::sync::Arc;
use spin::Mutex;
use libtime::get_rdtsc as rdtsc;
use crate::device::NvmeDev;

pub struct BlockReq {
    block: u64,
    num_blocks: u16,
    data: Vec<u8>,
}

impl BlockReq {
    pub fn new(block:u64 , num_blocks: u16, data: Vec<u8>) -> BlockReq {
        BlockReq {
            block,
            num_blocks,
            data,
        }
    }
}
impl Clone for BlockReq {
    fn clone(&self) -> Self {
       Self {
            block: self.block,
            num_blocks: self.num_blocks,
            data: self.data.clone(),
       }
    }
}

struct Nvme {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    device_initialized: bool,
    device: RefCell<Option<NvmeDev>>
}

impl Nvme {
    fn new() -> Nvme {
        Nvme {
            vendor_id: 0x8086,
            device_id: 0x0953,
            driver: pci_driver::PciDrivers::NvmeDriver,
            device_initialized: false,
            device: RefCell::new(None)
        }
    }

    fn active(&self) -> bool {
        self.device_initialized
    }
}

impl pci_driver::PciDriver for Nvme {
    fn probe(&mut self, bar_region: DeviceBarRegions) {
        match bar_region {
            DeviceBarRegions::Nvme(bar) => {
                println!("got nvme bar region");
                if let Ok(nvme_dev) = NvmeDev::new(bar) {
                    self.device_initialized = true;
                    self.device.replace(Some(nvme_dev));
                }
            }
            _ => { println!("Got unknown bar region") }
        }
    }

    fn get_vid(&self) -> u16 {
        self.vendor_id
    }

    fn get_did(&self) -> u16 {
        self.device_id
    }

    fn get_driver_type(&self) -> pci_driver::PciDrivers {
        self.driver
    }
}

#[no_mangle]
pub fn nvme_init(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) {
    libsyscalls::syscalls::init(s);

    println!("nvme_init: starting nvme driver domain");
    let mut nvme = Nvme::new();
    if let Err(_) = pci.pci_register_driver(&mut nvme, 0, None) {
        println!("WARNING: failed to register IXGBE driver");
    }

    Box::new(nvme);
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
