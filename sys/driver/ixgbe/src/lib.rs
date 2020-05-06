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

extern crate malloc;
extern crate alloc;
extern crate b2histogram;

#[macro_use]
use b2histogram::Base2Histogram;
use byteorder::{ByteOrder, BigEndian};

use libtime::sys_ns_loopsleep;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
#[macro_use]
use alloc::vec::Vec;
use core::panic::PanicInfo;
use syscalls::{Syscall, PCI, Heap};
use console::{println, print};
use pci_driver::DeviceBarRegions;
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;

pub use libsyscalls::errors::Result;
use crate::device::Intel8259x;
use core::cell::RefCell;
use protocol::UdpPacket;
use core::ptr;

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
    fn submit_and_poll(&mut self, mut packets: &mut VecDeque<Vec<u8>>, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> usize {
        let mut ret: usize = 0;
        if !self.device_initialized {
            return ret;
        }

        if let Some(device) = self.device.borrow_mut().as_mut() {
            let dev: &mut Intel8259x = device;
            ret = dev.device.submit_and_poll(&mut packets, &mut collect, tx, false);
            packets.append(&mut collect);
        }
        ret
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

const BATCH_SIZE: usize = 32;

fn run_tx_udp_test(dev: &Ixgbe, payload_sz: usize, mut debug: bool) {
    let batch_sz: usize = BATCH_SIZE;
    let mut packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
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

    for i in 0..batch_sz {
        packets.push_front(pkt.clone());
    }

    let mut append_rdtsc: u64 = 0;
    let mut count: u64 = 0;
    let mut alloc_count = 0;
    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let start = rdtsc();
        let end = rdtsc() + 15 * 2_600_000_000;

        loop{
            let ret = dev.device.submit_and_poll(&mut packets, &mut collect, true, debug);

            sum += ret;
    
            packets.append(&mut collect);


            if packets.len() == 0 {
                alloc_count += 1;
                for i in 0..batch_sz {
                    packets.push_front(pkt.clone());
                }
            }
            if rdtsc() > end {
                break;
            }
        }

        let elapsed = rdtsc() - start;
        if sum == 0 {
            sum += 1;
        }
        println!("==> tx batch {} : {} iterations took {} cycles (avg = {})", payload_sz, sum, elapsed, elapsed / sum as u64);
        dev.dump_stats();
        println!(" alloc_count {}", alloc_count * 32);
        println!("Reaped {} packets", dev.device.tx_poll(&mut collect));
    }
}

fn run_rx_udptest(dev: &Ixgbe, pkt_size: usize, debug: bool) {
    let pkt_size = 2048;
    let batch_sz: usize = BATCH_SIZE;
    let mut packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut collect: VecDeque<Vec<u8>> = VecDeque::new();

    for i in 0..batch_sz {
        packets.push_front(Vec::with_capacity(pkt_size));
    }

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let idev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let mut alloc_count = 0;

        let mut submit_rx_hist = Base2Histogram::new();
        let mut collect_rx_hist = Base2Histogram::new();

        let mut collect_start = true;
        let mut collect_end = false;
        let mut seq_start: u64 = 0;
        let mut seq_end: u64 = 0;

        let start = rdtsc();
        let end = start + 15 * 2_600_000_000;

        loop {
            submit_rx_hist.record(packets.len() as u64);
            let ret = idev.device.submit_and_poll(&mut packets, &mut collect, false, debug);
            if debug {
                println!("rx packets.len {} collect.len {} ret {}", packets.len(), collect.len(), ret);
            }
            sum += collect.len();
            collect_rx_hist.record(collect.len() as u64);

            if collect_start && !collect.is_empty() {
                let pkt = &collect[0];
                dump_packet(pkt);
                seq_start = BigEndian::read_u64(&pkt[42..42+8]);
                collect_start = false;
                collect_end = true;
            }

            packets.append(&mut collect);

            if rdtsc() > end {
                break;
            }

            if packets.len() < batch_sz / 4 {
                //println!("allocating new batch");
                alloc_count += 1;

                for i in 0..batch_sz {
                    packets.push_front(Vec::with_capacity(pkt_size));
                }
            }
        }

        let elapsed = rdtsc() - start;

        println!("rx packets.len {} collect.len {} ", packets.len(), collect.len());
        let ret = idev.device.submit_and_poll(&mut packets, &mut collect, false, false);
        if collect_end && !collect.is_empty() {
            let pkt = &collect[0];
            dump_packet(pkt);
            seq_end = BigEndian::read_u64(&pkt[42..42+8]);
        }

        println!("seq_start {} seq_end {} delta {}", seq_start, seq_end, seq_end - seq_start);
        println!("sum {} batch alloc_count {}", sum, alloc_count);
        println!("==> rx batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, sum, elapsed, elapsed / sum as u64);
        idev.dump_stats();
        for hist in alloc::vec![submit_rx_hist, collect_rx_hist] {
            println!("hist:");
            // Iterate buckets that have observations
            for bucket in hist.iter().filter(|b| b.count > 0) {
                print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
                print!("\n");
            }
        }

        println!("Reaped {} packets", idev.device.rx_poll(&mut collect));
    }
}

fn dump_packet(pkt: &Vec<u8>) {
    for (i, b) in pkt.iter().enumerate() {
        print!("{:02X} ", b); 

        if i > 0 && (i + 1) % 25 == 0 { 
            print!("\n");
        }
    }
    print!("\n");
}

fn run_fwd_udptest(dev: &Ixgbe, pkt_size: u16) {
    let batch_sz = BATCH_SIZE;
    let mut rx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut tx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();
    
    let mut sender_mac = alloc::vec![ 0x90, 0xe2, 0xba, 0xb3, 0x74, 0x81];
    let mut our_mac = alloc::vec![0x90, 0xe2, 0xba, 0xb5, 0x14, 0xcd];


    for i in 0..batch_sz {
        rx_packets.push_front(Vec::with_capacity(2048));
    }

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let dev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let mut fwd_sum: usize = 0;

        let start = rdtsc();
        let end = start + 30 * 2_600_000_000;

        let mut tx_elapsed = 0;
        let mut rx_elapsed = 0;

        let mut submit_rx: usize = 0;
        let mut submit_tx: usize = 0;
        let mut loop_count: usize = 0;

        loop {
            loop_count = loop_count.wrapping_add(1);

            submit_rx += rx_packets.len();
            submit_rx_hist.record(rx_packets.len() as u64);
            //println!("call rx_submit_poll packet {}", packets.len());
            let rx_start = rdtsc();
            let ret = dev.device.submit_and_poll(&mut rx_packets, &mut tx_packets, false, false);
            rx_elapsed += rdtsc() - rx_start;
            sum += ret;

            //println!("rx: submitted {} collect {}", ret, tx_packets.len());

            for pkt in tx_packets.iter_mut() {
                unsafe {
                    ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                    ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
                }
            }

            submit_tx += tx_packets.len();
            submit_tx_hist.record(tx_packets.len() as u64);
            let tx_start = rdtsc();
            let ret = dev.device.submit_and_poll(&mut tx_packets, &mut rx_packets, true, false);
            tx_elapsed += rdtsc() - tx_start;
            fwd_sum += ret;

            //print!("tx: submitted {} collect {}\n", ret, rx_packets.len());

            if rx_packets.len() == 0 && tx_packets.len() < batch_sz * 4 {
                //println!("-> Allocating new rx_ptx batch");
                for i in 0..batch_sz {
                    rx_packets.push_front(Vec::with_capacity(2048));
                }
            }

            if rdtsc() > end {
                break;
            }
        }

        let elapsed = rdtsc() - start;
        for hist in alloc::vec![submit_rx_hist, submit_tx_hist] {
            println!("hist:");
            // Iterate buckets that have observations
            for bucket in hist.iter().filter(|b| b.count > 0) {
                print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
                print!("\n");
            }
        }

        println!("Received {} forwarded {}", sum, fwd_sum);
        println!(" ==> submit_rx {} (avg {}) submit_tx {} (avg {}) loop_count {}",
                            submit_rx, submit_rx / loop_count, submit_tx, submit_tx / loop_count, loop_count);
        println!(" ==> rx batching {}B: {} packets took {} cycles (avg = {})",
                            pkt_size, sum, rx_elapsed, rx_elapsed  / sum as u64);
        println!(" ==> tx batching {}B: {} packets took {} cycles (avg = {})",
                            pkt_size, fwd_sum, tx_elapsed, tx_elapsed  / fwd_sum as u64);
        println!("==> fwd batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, fwd_sum, elapsed, elapsed / fwd_sum as u64);
        dev.dump_stats();
        //dev.dump_tx_descs();
    }
}

const ONE_MS_IN_NS: u64 = 1_000_000 * 1;

#[no_mangle]
pub fn ixgbe_init(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>,
                 pci: Box<dyn syscalls::PCI>) -> Box<dyn syscalls::Net> {
    libsyscalls::syscalls::init(s);

    println!("ixgbe_init: =>  starting ixgbe driver domain");
    let mut ixgbe = Ixgbe::new();
    if let Err(_) = pci.pci_register_driver(&mut ixgbe, 0, None) {
        println!("WARNING: failed to register IXGBE driver");
    }

    println!("Starting tests");

    let payload_sz = alloc::vec![64 - 42, 64, 128, 256, 512, 1470];

    println!("=> Running tests...");

    for p in payload_sz.iter() {
        println!("running {}B payload test", p);
        println!("Tx test");
        run_tx_udp_test(&ixgbe, *p, false);

        println!("Rx test");
        run_tx_udp_test(&ixgbe, *p, false);

        println!("Fwd test");
        run_fwd_udptest(&ixgbe, 64 - 42);
    }

    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_backtrace();
    loop {}
}
