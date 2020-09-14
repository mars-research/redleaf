#![no_std]
#![no_main]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
        maybe_uninit_extra
)]
//#![forbid(unsafe_code)]

mod device;
mod ixgbe_desc;
mod nullnet;
mod redhttpd;
mod smoltcp_device;

extern crate malloc;
extern crate alloc;
extern crate b2histogram;
extern crate sashstore_redleaf;

#[cfg(target_os = "linux")]
use error::plsbreakthebuild;

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

pub use usr::error::{ErrorKind, Result};
use crate::device::Intel8259x;
use core::cell::RefCell;
use protocol::UdpPacket;
use core::{mem, ptr};
use rref::{RRef, RRefDeque};
use libbenchnet::packettool;

pub use usr::net::NetworkStats;
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
            // SASHSTORE = Some(SashStore::with_capacity((1 << 20)));
            SASHSTORE = Some(SashStore::with_capacity(1 << 21));
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
        >, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        Ok((||{
            let mut ret: usize = 0;
            let device = &mut self.device.borrow_mut();
            let device = device.as_mut().ok_or(ErrorKind::UninitializedDevice)?;
            ret = device.device.submit_and_poll(&mut packets, &mut collect, tx, false);
            Ok(ret)
        })())       
    }

    fn submit_and_poll_rref(
        &self,
        mut packets: RRefDeque<[u8; 1514], 32>,
        mut collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>>
    {
        Ok((||{
            let mut ret: usize = 0;
    
            let mut packets = Some(packets);
            let mut collect = Some(collect);
    
            let device = &mut self.device.borrow_mut();
            let device = device.as_mut().ok_or(ErrorKind::UninitializedDevice)?;
            let (num, mut packets_, mut collect_) = device.device.submit_and_poll_rref(packets.take().unwrap(),
                                                    collect.take().unwrap(), tx, pkt_len, false);
            ret = num;
            packets.replace(packets_);
            collect.replace(collect_);

            // dev.dump_stats();
    
            Ok((ret, packets.unwrap(), collect.unwrap()))
        })())       
    }

    fn poll(&self, mut collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        Ok((||{
            let mut ret: usize = 0;
    
            let device = &mut self.device.borrow_mut();
            let device = device.as_mut().ok_or(ErrorKind::UninitializedDevice)?;
            ret = device.device.poll(&mut collect, tx);

            Ok(ret)
        })())       
    }

    fn poll_rref(&self, mut collect: RRefDeque<[u8; 1514], 512>, tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        Ok((||{
            let mut ret: usize = 0;
            let mut collect = Some(collect);
    
            let device = &mut self.device.borrow_mut();
            let device = device.as_mut().ok_or(ErrorKind::UninitializedDevice)?;
            let (num, mut collect_) = device.device.poll_rref(collect.take().unwrap(), tx);
            ret = num;
            collect.replace(collect_);
    
            Ok((ret, collect.unwrap()))
        })())       
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        Ok((||{
            let mut ret = NetworkStats::new();

            let device = &mut self.device.borrow_mut();
            let device = device.as_mut().ok_or(ErrorKind::UninitializedDevice)?;
            let stats = device.get_stats();
            ret = stats;

            Ok(ret) 
        })())       
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

fn run_sashstoretest(dev: &Ixgbe, pkt_size: u16) {
    let batch_sz = 32;
    let mut rx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut tx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();

    for i in 0..batch_sz {
        rx_packets.push_front(Vec::with_capacity(2048));
    }

    if let Some(device) = dev.device.borrow_mut().as_mut() {
        let idev: &mut Intel8259x = device;
        let mut sum: usize = 0;
        let mut fwd_sum: usize = 0;

        let start = rdtsc();
        let end = start + 60 * 2_600_000_000;

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
            let ret = idev.device.submit_and_poll(&mut rx_packets, &mut tx_packets, false, false);
            rx_elapsed += rdtsc() - rx_start;
            sum += ret;

            for mut pkt in tx_packets.iter_mut() {
                if let Some((padding, payload)) = packettool::get_mut_udp_payload(pkt) {
                    if let Some(mut sashstore) = unsafe { SASHSTORE.as_mut() } {
                        let payloadptr = payload as *mut _ as *mut u8;
                        let mut payloadvec = unsafe {
                            Vec::from_raw_parts(
                                payloadptr,
                                payload.len(),
                                2048 - padding, // FIXME: Awful
                            )
                        };

                        // println!("Before handle: payloadvec.capacity() = {}, len() = {}", payloadvec.capacity(), payloadvec.len());
                        let responsevec = unsafe { sashstore.handle_network_request(payloadvec) };

                        // assert!(responsevec.as_ptr() == payloadptr);
                        // println!("Handled: {:x?} -> {:x?}", responsevec.as_ptr(), payloadptr);
                        // println!("After handle: responsevec.capacity() = {}, len() = {}", responsevec.capacity(), responsevec.len());
                        if responsevec.as_ptr() != payloadptr {
                            unsafe {
                                ptr::copy(responsevec.as_ptr(), payloadptr, responsevec.len());
                            }
                        }

                        // println!("Before set_len: {}", pkt.len());
                        unsafe {
                            pkt.set_len(padding + responsevec.len());
                        }
                        // println!("After set_len: padding={}, resposevec.len() = {}, set to {}", padding, responsevec.len(), pkt.len());

                        packettool::swap_udp_ips(pkt);
                        packettool::swap_mac(pkt);
                        packettool::fix_ip_length(pkt);
                        packettool::fix_ip_checksum(pkt);
                        packettool::fix_udp_length(pkt);
                        packettool::fix_udp_checksum(pkt);

                        // println!("To send: {:x?}", pkt);
                    } else {
                        println!("No sashstore???");
                    }
                } else {
                    // println!("Not a UDP packet: {:x?}", &pkt);
                }
            }

            submit_tx += tx_packets.len();
            submit_tx_hist.record(tx_packets.len() as u64);
            let tx_start = rdtsc();
            let ret = idev.device.submit_and_poll(&mut tx_packets, &mut rx_packets, true, false);
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

        sashstore_redleaf::indexmap::print_stats();

        println!("Received {} forwarded {}", sum, fwd_sum);
        println!(" ==> submit_rx {} (avg {}) submit_tx {} (avg {}) loop_count {}",
                            submit_rx, submit_rx / loop_count, submit_tx, submit_tx / loop_count, loop_count);
        println!(" ==> rx batching {}B: {} packets took {} cycles (avg = {})",
                            pkt_size, sum, rx_elapsed, rx_elapsed  / sum as u64);
        println!(" ==> tx batching {}B: {} packets took {} cycles (avg = {})",
                            pkt_size, fwd_sum, tx_elapsed, tx_elapsed  / fwd_sum as u64);
        println!("==> fwd batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, fwd_sum, elapsed, elapsed / fwd_sum as u64);
        idev.dump_stats();
        //dev.dump_tx_descs();
    }
}

#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>,
                 pci: Box<dyn usr::pci::PCI>) -> Box<dyn usr::net::Net> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("ixgbe_init: =>  starting ixgbe driver domain");
    #[cfg(not(feature = "nullnet"))]
    let mut ixgbe = {
        let mut ixgbe = Ixgbe::new();
        if let Err(_) = pci.pci_register_driver(&mut ixgbe, 0, None) {
            println!("WARNING: failed to register IXGBE driver");
        }
        ixgbe
    };

    #[cfg(feature = "nullnet")]
    let mut ixgbe = nullnet::NullNet::new();

    println!("Starting tests");

   /* 
    for _ in 0..5 {
        libbenchnet::run_tx_udptest_rref(&ixgbe, 64, false);
    }*/
    
    /*for _ in 0..5 {
        libbenchnet::run_tx_udptest(&ixgbe, 64, false);
    }*/

    //run_sashstoretest(&ixgbe, 64);

    /*
    for _ in 0..5 {
        libbenchnet::run_rx_udptest_with_delay(&ixgbe, 64, false, 0);
    }*/

    /*for _ in 0..5 {
        libbenchnet::run_fwd_udptest(&ixgbe, 64);
    }*/
    /*
    for d in (0..=1000).step_by(100) {
        libbenchnet::run_fwd_udptest_with_delay(&ixgbe, 64, d);
    }

    panic!("");
    */
    //libbenchnet::run_fwd_udptest_rref(&ixgbe, 1514);

    // libbenchnet::run_maglev_fwd_udptest_rref(&ixgbe, 64);

    /*
    let payload_sz = alloc::vec![64 - 42, 64, 128, 256, 512, 1470];
    println!("=> Running tests...");

    for p in payload_sz.iter() {
        println!("running {}B payload test", p);
        println!("Tx test");
        run_tx_udptest(&ixgbe, *p, false);

        println!("Rx test");
        run_rx_udptest(&ixgbe, *p, false);

        println!("Fwd test");
        run_fwd_udptest(&ixgbe, 64 - 42);
    }
    */

    Box::new(ixgbe)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
