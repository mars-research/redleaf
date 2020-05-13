#![no_std]
extern crate malloc;
extern crate alloc;
use libsyscalls;
use syscalls::{Syscall, Heap};
use create;
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::{println, print};
use core::alloc::Layout;
use core::panic::PanicInfo;
use b2histogram::Base2Histogram;
use usr;
use rref::{RRef, RRefDeque};
use alloc::vec::Vec;
use usr::net::Net;

#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, mut net: Box<dyn Net + Send>) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("Init domain benchnet_inside");

    run_tx_udptest_rref(net, 22, false);
}

const BATCH_SIZE: usize = 32;

fn run_tx_udptest_rref(mut net: Box<dyn Net + Send>, payload_sz: usize, mut debug: bool) {
    let batch_sz: usize = BATCH_SIZE;
    let mut packets = RRefDeque::<[u8; 1512], 32>::new(Default::default());
    let mut collect = RRefDeque::<[u8; 1512], 32>::new(Default::default());
   // let mut poll =  RRefDeque::<[u8; 1512], 512>::new(Default::default());

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

    let len = pkt.len();
    if len < 1512 {
        let pad = alloc::vec![0u8; 1512 - len];
        pkt.extend(pad.iter());
    }

    let mut pkt_arr = [0; 1512];

    println!("pkt.len {} pkt_arr.len {}", pkt.len(), pkt_arr.len());

    pkt_arr.copy_from_slice(pkt.as_slice());

    for i in 0..batch_sz {
        packets.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
    }


    //for i in 0..512 {
        //poll.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
    //}

    let mut append_rdtsc: u64 = 0;
    let mut count: u64 = 0;
    let mut alloc_count = 0;

    let mut packets = Some(packets);
    let mut collect = Some(collect);

    let mut collect_tx_hist = Base2Histogram::new();
    //let mut poll = Some(poll);

    let mut sum: usize = 0;
    let start = libtime::get_rdtsc();
    let end = libtime::get_rdtsc() + 15 * 2_600_000_000;

    loop{
        let (ret, mut packets_, mut collect_) = net.submit_and_poll_rref(packets.take().unwrap(),
                                collect.take().unwrap(), true);
        sum += ret;

        // println!("ret {}", ret);

        collect_tx_hist.record(collect_.len() as u64);

        while let Some(packet) = collect_.pop_front() {
            packets_.push_back(packet);
        }

        if packets_.len() == 0 {
            alloc_count += 1;
            for i in 0..batch_sz {
                packets_.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
            }
        }
        if libtime::get_rdtsc() > end {
            break;
        }

        packets.replace(packets_);
        collect.replace(collect_);
    }

    let elapsed = libtime::get_rdtsc() - start;
    if sum == 0 {
        sum += 1;
    }
    println!("==> tx batch {} : {} iterations took {} cycles (avg = {})", payload_sz, sum, elapsed, elapsed / sum as u64);
    // dev.dump_stats();
    println!(" alloc_count {}", alloc_count * 32);
    //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
    //println!("Reaped {} packets", dev.device.tx_poll_rref(poll.take().unwrap()).0);
    for hist in alloc::vec![collect_tx_hist] {
        println!("hist:");
        // Iterate buckets that have observations
        for bucket in hist.iter().filter(|b| b.count > 0) {
            print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
            print!("\n");
        }
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

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
