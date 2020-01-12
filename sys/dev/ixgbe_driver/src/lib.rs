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
    maybe_uninit_extra
)]

mod device;
mod dma;
mod ixgbe_desc;

extern crate malloc;
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall,PCI};
use libsyscalls::syscalls::{sys_println, sys_alloc, sys_create_thread};
use console::println;
use pci_driver::BarRegions;
use ixgbe::IxgbeBarRegion;
use core::mem::MaybeUninit;
pub use libsyscalls::errors::Result;
use crate::device::Intel8259x;
use core::cell::RefCell;
use protocol::UdpPacket;
use alloc::sync::Arc;
use spin::Mutex;

struct Ixgbe {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    device_initialized: bool,
    device: RefCell<Option<Intel8259x>>
}

/*struct IxgbeBar<'a> {
    ixgbe_bar: &'a dyn IxgbeBarRegion,
}*/

impl Ixgbe {
    fn new() -> Ixgbe {
        Ixgbe {
            vendor_id: 0x8086,
            device_id: 0x10fb,
            driver: pci_driver::PciDrivers::IxgbeDriver,
            device_initialized: false,
            device: RefCell::new(None)
        }
    }

    fn active(&self) -> bool {
        self.device_initialized
    }
}
/*
static mut ixgbe_bar: MaybeUninit<IxgbeBar> = MaybeUninit::uninit();

impl<'a> IxgbeBar<'a> {
    fn new(bar: &'a dyn IxgbeBarRegion) -> IxgbeBar<'a> {
        IxgbeBar {
            ixgbe_bar: bar
        }
    }
}
*/
impl syscalls::Net for Ixgbe {
    fn send(&self, buf: &[u8]) -> u32 {
        if self.device_initialized == false {
            0
        } else {
            if self.active() {
                if let Some(mut device) = self.device.borrow_mut().as_mut() {
                    let dev: &mut Intel8259x = device;
                    if let Ok(Some(opt)) = dev.write(buf) {
                        opt as u32
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        }
    }

    fn send_udp(&self, packet: Arc<Mutex<UdpPacket>>) -> u32 {
         if self.device_initialized == false {
            0
        } else {
            if self.active() {
                if let Some(mut device) = self.device.borrow_mut().as_mut() {
                    let dev: &mut Intel8259x = device;
                    let mut ret: u32 = 0;
                    for i in 0..100_000 {
                        if let Ok(Some(opt)) = dev.write(packet.lock().as_slice()) {
                            ret = opt as u32;
                        } else {
                            ret = 0;
                        }
                    }
                    ret
                } else {
                    0
                }
            } else {
                0
            }
        }
    }
}

impl pci_driver::PciDriver for Ixgbe {
    fn probe(&mut self, bar_region: BarRegions) {
        match bar_region {
            BarRegions::Ixgbe(bar) => {
                //let ixgbebar = IxgbeBar::new(bar.as_ref());
                println!("got ixgbe bar region");
                /*unsafe {
                    ixgbe_bar.write(ixgbebar);
                }*/
                if let Ok(ixgbe_dev) = Intel8259x::new(bar) {
                    self.device_initialized = true;
                    self.device.replace(Some(ixgbe_dev));
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
pub fn ixgbe_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net> {
    libsyscalls::syscalls::init(s);

    println!("ixgbe_init: starting ixgbe driver domain");
    let mut ixgbe = Ixgbe::new();
    pci.pci_register_driver(&mut ixgbe, 0);
    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
