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
//#![forbid(unsafe_code)]

mod device;
mod ixgbe_desc;
mod smoltcp_device;
mod redhttpd;

extern crate malloc;
extern crate alloc;
extern crate b2histogram;
extern crate sashstore_redleaf;

#[macro_use]
use b2histogram::Base2Histogram;
use byteorder::{ByteOrder, BigEndian};

use libtime::sys_ns_loopsleep;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
#[macro_use]
use alloc::vec::Vec;
use alloc::vec;
use core::panic::PanicInfo;
use syscalls::{Syscall, Heap};
use usr;
use usr::rpc::RpcResult;
use console::{println, print};
use pci_driver::DeviceBarRegions;
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;

pub use libsyscalls::errors::Result;
use crate::device::Intel8259x;
use core::cell::RefCell;
use protocol::UdpPacket;
use core::{mem, ptr};
use rref::{RRef, RRefDeque};
use libbenchnet::packettool;

use libtime::get_rdtsc as rdtsc;

use sashstore_redleaf::SashStore;

static mut SASHSTORE: Option<SashStore> = None;

struct Ixgbe {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    device_initialized: bool,
    device: RefCell<Option<Intel8259x>>,
}

impl Ixgbe {
    fn new() -> Ixgbe {
        unsafe {
            SASHSTORE = Some(SashStore::with_capacity(1));
        }

        Ixgbe {
            vendor_id: 0x8086,
            device_id: 0x10fb,
            driver: pci_driver::PciDrivers::IxgbeDriver,
            device_initialized: false,
            device: RefCell::new(None),
        }
    }

    fn active(&self) -> bool {
        self.device_initialized
    }
}

impl usr::net::Net for Ixgbe {
    fn submit_and_poll(&self, mut packets: &mut VecDeque<Vec<u8>
        >, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<usize> {
        let mut ret: usize = 0;
        if !self.device_initialized {
            return Ok(ret);
        }

        if let Some(device) = self.device.borrow_mut().as_mut() {
            let dev: &mut Intel8259x = device;
            ret = dev.device.submit_and_poll(&mut packets, &mut collect, tx, false);
            packets.append(&mut collect);
        }
        Ok(ret)
    }

    fn submit_and_poll_rref(
        &self,
        mut packets: RRefDeque<[u8; 1512], 32>,
        mut collect: RRefDeque<[u8; 1512], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<(
            usize,
            RRefDeque<[u8; 1512], 32>,
            RRefDeque<[u8; 1512], 32>
        )>
    {

        let mut ret: usize = 0;
        if !self.device_initialized {
            return Ok((ret, packets, collect));
        }

        let mut packets = Some(packets);
        let mut collect = Some(collect);

        if let Some(device) = self.device.borrow_mut().as_mut() {
            let dev: &mut Intel8259x = device;
            let (num, mut packets_, mut collect_) = dev.device.submit_and_poll_rref(packets.take().unwrap(),
                                                    collect.take().unwrap(), tx, pkt_len, false);
            ret = num;
            packets.replace(packets_);
            collect.replace(collect_);

            // dev.dump_stats();
        }

        Ok((ret, packets.unwrap(), collect.unwrap()))
    }

    fn poll(&self, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<usize> {
        let mut ret: usize = 0;
        if !self.device_initialized {
            return Ok(ret);
        }

        if let Some(device) = self.device.borrow_mut().as_mut() {
            let dev: &mut Intel8259x = device;
            ret = dev.device.poll(&mut collect, tx);
        }
        Ok(ret)
    }

    fn poll_rref(&self, mut collect: RRefDeque<[u8; 1512], 512>, tx: bool) -> RpcResult<(usize, RRefDeque<[u8; 1512], 512>)> {
        let mut ret: usize = 0;
        if !self.device_initialized {
            return Ok((ret, collect));
        }

        let mut collect = Some(collect);

        if let Some(device) = self.device.borrow_mut().as_mut() {
            let dev: &mut Intel8259x = device;
            let (num, mut collect_) = dev.device.poll_rref(collect.take().unwrap(), tx);
            ret = num;
            collect.replace(collect_);
        }

        Ok((ret, collect.unwrap()))
    }

}

impl pci_driver::PciDriver for Ixgbe {
    fn probe(&mut self, bar_region: DeviceBarRegions) {
        println!("ixgbe probe called");
        match bar_region {
            DeviceBarRegions::Ixgbe(bar) => {
                println!("got ixgbe bar region");
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
                 heap: Box<dyn Heap + Send + Sync>,
                 pci: Box<dyn usr::pci::PCI>) -> Box<dyn usr::net::Net + Send> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("ixgbe_init: =>  starting ixgbe driver domain");
    let mut ixgbe = Ixgbe::new();
    if let Err(_) = pci.pci_register_driver(&mut ixgbe, 0, None) {
        println!("WARNING: failed to register IXGBE driver");
    }

    println!("Starting tests");

    let payload_sz = alloc::vec![64 - 42, 64, 128, 256, 512, 1470];

    // run_tx_udptest(&ixgbe, 64, false);

    libbenchnet::run_tx_udptest_rref(&ixgbe, 64, false);

    // run_rx_udptest(&ixgbe, 64, false);

    libbenchnet::run_rx_udptest_rref(&ixgbe, 64, false);

    // run_fwd_udptest(&ixgbe, 64);

    libbenchnet::run_fwd_udptest_rref(&ixgbe, 64);

    libbenchnet::run_maglev_fwd_udptest_rref(&ixgbe, 64);

    /*println!("=> Running tests...");

    for p in payload_sz.iter() {
        println!("running {}B payload test", p);
        println!("Tx test");
        run_tx_udptest(&ixgbe, *p, false);

        println!("Rx test");
        run_rx_udptest(&ixgbe, *p, false);

        println!("Fwd test");
        run_fwd_udptest(&ixgbe, 64 - 42);
    }*/

    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_backtrace();
    loop {}
}
