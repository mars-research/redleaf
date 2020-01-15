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
#[macro_use]
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
use protocol::{UdpPacket, MTU_SZ};
use alloc::sync::Arc;
use spin::Mutex;
use libsyscalls::time::get_rdtsc as rdtsc;

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

fn calc_ipv4_checksum(ipv4_header: &[u8]) -> u16 {
    assert!(ipv4_header.len() % 2 == 0);
    let mut checksum = 0;
    for i in 0..ipv4_header.len() / 2 {
        if i == 5 {
            // Assume checksum field is set to 0
            continue;
        }
        checksum += (u32::from(ipv4_header[i * 2]) << 8) + u32::from(ipv4_header[i * 2 + 1]);
        if checksum > 0xffff {
            checksum = (checksum & 0xffff) + 1;
        }
    }
    !(checksum as u16)
}

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

    fn send_udp_from_ixgbe(&self, packet: &[u8]) -> u32 {
        let mac_data = [
            0x90, 0xe2, 0xba, 0xb3, 0xb9, 0x50, // Dst mac
            0x90, 0xe2, 0xba, 0xb5, 0x14, 0xf5, // Src mac
            0x08, 0x00,                         // Protocol
        ];
        let mut ip_data = [
            //0x45, 0x00, 0x05, 0xdc, 0x78, 0xb4, 0x40, 0x00,
            0x45, 0x00,
            0x00,
            0x2e,
            0x00, 0x0, 0x0, 0x00,
            0x40, 0x11, 0x00, 0x00,
            0x0a, 0x0a, 0x03, 0x01,
            0x0a, 0x0a, 0x03, 0x02,
        ];

        let udp_hdr = [
            0xb2, 0x6f, 0x14, 0x51,
            0x00,
            0x1a,
            0x9c, 0xaf,
        ];

        let mut payload = [
            b'R', b'e', b'd', b'l', b'e', b'a', b'f', 0x0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0,0,
        ];


        let checksum = calc_ipv4_checksum(&ip_data);
        // Calculated checksum is little-endian; checksum field is big-endian
        ip_data[10] = (checksum >> 8) as u8;
        ip_data[11] = (checksum & 0xff) as u8;

        let eth_hdr = protocol::EthernetHeader(mac_data);
        let ip_hdr = protocol::IpV4Header(ip_data);
        let udp_hdr = protocol::UdpHeader(udp_hdr);
        let payload = payload;
        let pkt = UdpPacket::new(eth_hdr, ip_hdr, udp_hdr, payload);

        let mut pvec: Vec<UdpPacket> = Vec::with_capacity(32);

        for i in 0..32 {
            pvec.push(UdpPacket::new_zeroed());
            unsafe {
                core::ptr::copy(
                    pkt.as_slice() as *const _ as *const u8,
                    &mut pvec[i] as *mut UdpPacket as *mut u8,
                    MTU_SZ);
            }
        }

        if self.device_initialized == false {
            0
        } else {
            if self.active() {
                if let Some(mut device) = self.device.borrow_mut().as_mut() {
                    let dev: &mut Intel8259x = device;
                    let mut ret: usize = 0;
                    let start = rdtsc();
                    for i in 0..5_00_000 {
                        ret += dev.tx_batch(&pvec);
                    }
                    let end = rdtsc();
                    println!("From ixgbe layer: {} iterations took {} cycles (avg = {})", 20_000_000, end-start, (end - start) / ret as u64);

                    ret as u32
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
                    if let Ok(Some(opt)) = dev.write(packet.lock().as_slice()) {
                        ret = opt as u32;
                    } else {
                        ret = 0;
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

struct UdpQueue {
    udp_packet: UdpPacket,
    sent: bool,
}

fn run_udp_test(dev: &Ixgbe) {
    let mac_data = [
        0x90, 0xe2, 0xba, 0xb3, 0xb9, 0x50, // Dst mac
        0x90, 0xe2, 0xba, 0xb5, 0x14, 0xf5, // Src mac
        0x08, 0x00,                         // Protocol
    ];
    let mut ip_data = [
        //0x45, 0x00, 0x05, 0xdc, 0x78, 0xb4, 0x40, 0x00,
        0x45, 0x00,
        0x00,
        0x2e,
        0x00, 0x0, 0x0, 0x00,
        0x40, 0x11, 0x00, 0x00,
        0x0a, 0x0a, 0x03, 0x01,
        0x0a, 0x0a, 0x03, 0x02,
    ];

    let udp_hdr = [
        0xb2, 0x6f, 0x14, 0x51,
        0x00,
        0x1a,
        0x9c, 0xaf,
    ];

    let mut payload = [
        b'R', b'e', b'd', b'l', b'e', b'a', b'f', 0x0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0,0,
    ];


    let checksum = calc_ipv4_checksum(&ip_data);
    // Calculated checksum is little-endian; checksum field is big-endian
    ip_data[10] = (checksum >> 8) as u8;
    ip_data[11] = (checksum & 0xff) as u8;

    let eth_hdr = protocol::EthernetHeader(mac_data);
    let ip_hdr = protocol::IpV4Header(ip_data);
    let udp_hdr = protocol::UdpHeader(udp_hdr);
    let payload = payload;
    let pkt = UdpPacket::new(eth_hdr, ip_hdr, udp_hdr, payload);

    let mut pvec: Vec<UdpPacket> = Vec::with_capacity(32);

    for i in 0..32 {
        pvec.push(UdpPacket::new_zeroed());
        unsafe {
            core::ptr::copy(
                pkt.as_slice() as *const _ as *const u8,
                &mut pvec[i] as *mut UdpPacket as *mut u8,
                MTU_SZ);
        }
    }

    println!("{:?}", pvec[0]);

    if let Some(mut device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let start = rdtsc();
        while sum <= 20_000_000 {
            let ret = dev.tx_batch(&pvec);
            sum += ret;
        }
        println!("sum {}", sum);
        let elapsed = rdtsc() - start;
        println!("==> tx batch : {} iterations took {} cycles (avg = {})", sum, elapsed, elapsed / sum as u64);
        dev.dump_stats();
    }
}

#[no_mangle]
pub fn ixgbe_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net> {
    libsyscalls::syscalls::init(s);

    println!("ixgbe_init: starting ixgbe driver domain");
    let mut ixgbe = Ixgbe::new();
    pci.pci_register_driver(&mut ixgbe, 0);
    run_udp_test(&ixgbe);
    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
