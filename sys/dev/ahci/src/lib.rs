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

mod ahcid;

use core::panic::PanicInfo;
use syscalls::{Syscall};
use libsyscalls::syscalls::{sys_print, sys_alloc};
use console::println;
use alloc::boxed::Box;
use alloc::vec::Vec;

struct AHCI {
    disk: Box<dyn self::ahcid::Disk>,
}

impl AHCI {
    fn new(disk: Box<dyn self::ahcid::Disk>) -> AHCI {
        AHCI {
            disk: disk,
        }
    }
}

impl syscalls::BDev for AHCI {
    fn read(&self, block: u32, data: &mut [u8; 512]) {
        // self.disk.read(block as u64, data);
    }
    fn write(&self, block: u32, data: &[u8; 512]) {
        // self.disk.write(block as u64, data);
    }
}


#[no_mangle]
pub fn ahci_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::BDev> {
    libsyscalls::syscalls::init(s);

    let (hba, mut disks) = self::ahcid::disks(0xfebf1000, "meow");

    println!("ahci_init: Started AHCI domain");
    Box::new(AHCI::new(disks.remove(0))) 
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
