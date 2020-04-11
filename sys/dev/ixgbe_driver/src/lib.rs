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
#![forbid(unsafe_code)]

mod device;
mod ixgbe_desc;

extern crate malloc;
extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
#[macro_use]
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall,PCI};
use console::{println, print};
use pci_driver::BarRegions;
use libsyscalls::syscalls::sys_backtrace;

pub use libsyscalls::errors::Result;
use crate::device::Intel8259x;
use core::cell::RefCell;
use protocol::UdpPacket;

use libtime::get_rdtsc as rdtsc;

struct Ixgbe {
    vendor_id: u16,
    device_id: u16,
    driver: pci_driver::PciDrivers,
    device_initialized: bool,
    device: RefCell<Option<Intel8259x>>
}

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
        /*if !self.device_initialized {
            0
        } else if self.active() {
            if let Some(device) = self.device.borrow_mut().as_mut() {
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
        */
        0
    }

    fn send_udp_from_ixgbe(&self, _packet: &[u8]) -> u32 {
        /*const PAYLOAD_SZ: usize = 64;
        let mac_data = [
            0x90, 0xe2, 0xba, 0xb3, 0x74, 0x81, // Dst mac
            0x90, 0xe2, 0xba, 0xac, 0x16, 0x58, // Src mac
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

        let mut payload: [u8; PAYLOAD_SZ] = [0u8; PAYLOAD_SZ];
        payload[0] = b'R';
        payload[1] = b'e';
        payload[2] = b'd';
        payload[3] = b'l';
        payload[4] = b'e';
        payload[5] = b'a';
        payload[6] = b'f';

        let checksum = calc_ipv4_checksum(&ip_data);
        // Calculated checksum is little-endian; checksum field is big-endian
        ip_data[10] = (checksum >> 8) as u8;
        ip_data[11] = (checksum & 0xff) as u8;

        let eth_hdr = protocol::EthernetHeader(mac_data);
        let ip_hdr = protocol::IpV4Header(ip_data);
        let udp_hdr = protocol::UdpHeader(udp_hdr);
        let payload = payload;
        let pkt = UdpPacket::new(eth_hdr, ip_hdr, udp_hdr, payload);

        let mut pvec: Vec<UdpPacket<[u8; PAYLOAD_SZ]>> = Vec::with_capacity(32);

        for i in 0..32 {
            pvec.push(UdpPacket::new_zeroed(payload));
            unsafe {
                core::ptr::copy(
                    pkt.as_slice() as *const _ as *const u8,
                    &mut pvec[i] as *mut UdpPacket<[u8; PAYLOAD_SZ]> as *mut u8,
                    core::mem::size_of::<UdpPacket<[u8; PAYLOAD_SZ]>>());
            }
        }

        if !self.device_initialized {
            0
        } else if self.active() {
            if let Some(device) = self.device.borrow_mut().as_mut() {
                let dev: &mut Intel8259x = device;
                let mut ret: usize = 0;
                let start = rdtsc();
                for _i in 0..500_000 {
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
    */
        0
    }

    /*
    fn send_udp<T>(&self, packet: Arc<Mutex<UdpPacket<T>>>) -> u32 {
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
    */
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

fn run_tx_udp_test(dev: &Ixgbe, payload_sz: usize) {
    let mut packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(32);
    let mut collect: VecDeque<Vec<u8>> = VecDeque::new();

    let mac_data = alloc::vec![
        0x90, 0xe2, 0xba, 0xb3, 0x74, 0x81, // Dst mac
        0x90, 0xe2, 0xba, 0xb5, 0x14, 0xcd, // Src mac
        0x08, 0x00,                         // Protocol
    ];
    let mut ip_data = alloc::vec![
        0x45, 0x00,
        0x00,
        0x2e,
        0x00, 0x0, 0x0, 0x00,
        0x40, 0x11, 0x00, 0x00,
        0x0a, 0x0a, 0x03, 0x01,
        0x0a, 0x0a, 0x03, 0x02,
    ];

    let udp_hdr = alloc::vec![
        0xb2, 0x6f, 0x14, 0x51,
        0x00,
        0x1a,
        0x9c, 0xaf,
    ];

    let mut payload = alloc::vec![0u8; payload_sz];

    payload[0] = b'R';
    payload[1] = b'e';
    payload[2] = b'd';
    payload[3] = b'l';
    payload[4] = b'e';
    payload[5] = b'a';
    payload[6] = b'f';

    let checksum = calc_ipv4_checksum(&ip_data);
    // Calculated checksum is little-endian; checksum field is big-endian
    ip_data[10] = (checksum >> 8) as u8;
    ip_data[11] = (checksum & 0xff) as u8;

    let mut pkt:Vec<u8> = Vec::new();
    pkt.extend(mac_data.iter());
    pkt.extend(ip_data.iter());
    pkt.extend(udp_hdr.iter());
    pkt.extend(payload.iter());

    println!("Packet len is {}", pkt.len());

    for i in 0..32 {
        packets.push_front(pkt.clone());
    }

    let mut append_rdtsc: u64 = 0;
    let mut count: u64 = 0;
    let mut alloc_count = 0;
    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let start = rdtsc();
        while sum <= 20_000_000 {
            let ret = dev.device.submit_and_poll(&mut packets, &mut collect, true);
            sum += ret;

            packets.append(&mut collect);


            if packets.len() == 0 {
                alloc_count += 1;
                for i in 0..32 {
                    packets.push_front(pkt.clone());
                }
            }
        }

        let elapsed = rdtsc() - start;
        println!("==> tx batch {} : {} iterations took {} cycles (avg = {})", payload_sz, sum, elapsed, elapsed / sum as u64);
        dev.dump_stats();
        println!("Reaped {} packets", dev.device.poll(&mut collect));
    }
}

/*
use ixgbe::{IxgbeRegs};

fn run_read_reg_test(dev: &Ixgbe) {

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let start = rdtsc();
        while sum <= 20_000_000 {
            dev.bar.read_reg(IxgbeRegs::Gptc);
            sum += 1;
        }
        let elapsed = rdtsc() - start;
        println!("==> read_reg test: {} iterations took {} cycles (avg = {})", sum, elapsed, elapsed / sum as u64);
    }
}

fn run_write_reg_test(dev: &Ixgbe) {

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let start = rdtsc();
        while sum <= 20_000_000 {
            dev.bar.write_reg_tdt(0, 0);
            sum += 1;
        }
        let elapsed = rdtsc() - start;
        println!("==> write_reg test: {} iterations took {} cycles (avg = {})", sum, elapsed, elapsed / sum as u64);
    }
}

fn run_rx_udptest_64(dev: &Ixgbe) {
    const PAYLOAD_SZ: usize = 2048;
    let mut payload: [u8; PAYLOAD_SZ] = [0u8; PAYLOAD_SZ];

    let mut pvec: Vec<UdpPacket<[u8; PAYLOAD_SZ]>> = Vec::with_capacity(32);

    for i in 0..32 {
        pvec.push(UdpPacket::new_zeroed(payload));
    }

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let start = rdtsc();

        while sum <= 20_000_000 {
            let ret = dev.rx_batch(&mut pvec);
            sum += ret;
        }

        let elapsed = rdtsc() - start;
        println!("sum {}", sum);
        println!("==> rx batch MTU: {} iterations took {} cycles (avg = {})", sum, elapsed, elapsed / sum as u64);
        dev.dump_stats();
    }

}*/

fn run_rx_udptest(dev: &Ixgbe, pkt_size: u16) {
    let mut packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(32);
    let mut collect: VecDeque<Vec<u8>> = VecDeque::new();

    for i in 0..32 {
        packets.push_front(Vec::with_capacity(2048));
    }

    //println!("Sent a packet!");
    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let idev: &mut Intel8259x = device;
        let mut sum: usize = 0;

        let start = rdtsc();

        while sum <= 20_000_000 {
            let ret = idev.device.submit_and_poll(&mut packets, &mut collect, false);
            sum += ret;

            packets.append(&mut collect);

            if packets.len() == 0 {
                for i in 0..32 {
                    packets.push_front(Vec::with_capacity(2048));
                }
            }
        }

        let elapsed = rdtsc() - start;
        println!("sum {}", sum);
        println!("==> rx batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, sum, elapsed, elapsed / sum as u64);
        idev.dump_stats();
    }
}

const ONE_MS_IN_NS: u64 = 1_000_000 * 1;

#[no_mangle]
pub fn ixgbe_init(s: Box<dyn Syscall + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net> {
    libsyscalls::syscalls::init(s);

    println!("ixgbe_init: starting ixgbe driver domain");
    let mut ixgbe = Ixgbe::new();
    if let Err(_) = pci.pci_register_driver(&mut ixgbe, 0, None) {
        println!("WARNING: failed to register IXGBE driver");
    }

    println!("Starting tests");

    let payload_sz = alloc::vec![64 - 42, 64, 128, 256, 512, 1470];

    for p in payload_sz.iter() {
        println!("running {}B payload test", p);
        run_tx_udp_test(&ixgbe, *p);
    }

    //println!("running Rx test");

    //println!("running 64B rx test");
    //run_rx_udptest(&ixgbe, 64 - 42);

    println!("running 128B rx test");
    run_rx_udptest(&ixgbe, 128);
/*
    println!("running 256B rx test");
    run_rx_udptest(&ixgbe, 256);

    println!("running 512B rx test");
    run_rx_udptest(&ixgbe, 512);

    println!("running 1512B rx test");
    run_rx_udptest(&ixgbe, 1512);

    */

    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_backtrace();
    loop {}
}
