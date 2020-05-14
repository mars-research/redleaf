#![no_std]
#![forbid(unsafe_code)]
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

extern crate malloc;
extern crate alloc;

#[macro_use]
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use b2histogram::Base2Histogram;
use core::panic::PanicInfo;

use rref::{RRef, RRefDeque};
use syscalls::{Syscall, Heap};
use usrlib::{println, print};
use usrlib::syscalls::{sys_open, sys_fstat, sys_read, sys_write, sys_close};
use usr::xv6::Xv6;
use usr::vfs::{VFSPtr, DirectoryEntry, DirectoryEntryRef, INodeFileType, FileMode};

#[no_mangle]
// TODO: rv6 should be const not mut
pub fn init(s: Box<dyn Syscall + Send + Sync>, heap: Box<dyn Heap + Send + Sync>, mut rv6: Box<dyn Xv6 + Send + Sync>, args: &str) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone());
    println!("Starting rv6 benchnet with args: {}", args);

    run_tx_udptest_rref(&mut rv6, 22, false);
    run_rx_udptest_rref(&mut rv6, 22, false);
}

const BATCH_SIZE: usize = 32;

fn run_tx_udptest_rref(rv6: &mut Box<dyn Xv6 + Send + Sync>, payload_sz: usize, mut debug: bool) {
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
    let start = rv6.sys_rdtsc();
    let end = rv6.sys_rdtsc() + 15 * 2_600_000_000;

    loop{
        let (ret, mut packets_, mut collect_) = rv6.submit_and_poll_rref(packets.take().unwrap(),
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
        if rv6.sys_rdtsc() > end {
            break;
        }

        packets.replace(packets_);
        collect.replace(collect_);
    }

    let elapsed = rv6.sys_rdtsc() - start;
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

fn run_rx_udptest_rref(rv6: &mut Box<dyn Xv6 + Send + Sync>, pkt_size: usize, debug: bool) {
    let pkt_size = 2048;
    let batch_sz: usize = BATCH_SIZE;
    let mut packets = RRefDeque::<[u8; 1512], 32>::default();
    let mut collect = RRefDeque::<[u8; 1512], 32>::default();
    let mut poll =  RRefDeque::<[u8; 1512], 512>::default();

    let mut pkt_arr = [0; 1512];

    for i in 0..batch_sz {
        packets.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
    }

    let mut packets = Some(packets);
    let mut collect = Some(collect);
    let mut poll = Some(poll);

    println!("run_rx_udptest_rref");

    let mut sum: usize = 0;
    let mut alloc_count = 0;

    let mut submit_rx_hist = Base2Histogram::new();
    let mut collect_rx_hist = Base2Histogram::new();

    let mut collect_start = true;
    let mut collect_end = false;
    let mut seq_start: u64 = 0;
    let mut seq_end: u64 = 0;

    let start = rv6.sys_rdtsc();
    let end = start + 15 * 2_600_000_000;

    loop {
        //submit_rx_hist.record(packets.len() as u64);

        let (ret, mut packets_, mut collect_) = rv6.submit_and_poll_rref(packets.take().unwrap(),
                                collect.take().unwrap(), false);

        //if debug {
            //println!("rx packets.len {} collect.len {} ret {}", packets.len(), collect.len(), ret);
        //}
        sum += collect_.len();
        collect_rx_hist.record(collect_.len() as u64);

        //if collect_start && !collect.is_empty() {
            //let pkt = &collect[0];
            //dump_packet(pkt);
            //seq_start = BigEndian::read_u64(&pkt[42..42+8]);
            //collect_start = false;
            //collect_end = true;
        //}

        //packets.append(&mut collect);

        while let Some(packet) = collect_.pop_front() {
            packets_.push_back(packet);
        }

        if rv6.sys_rdtsc() > end {
            break;
        }

        //if packets_.len() < batch_sz / 4 {
        if packets_.len() == 0 {
            let alloc_sz = batch_sz - packets_.len();
            //println!("allocating new batch");
            alloc_count += 1;

            for i in 0..alloc_sz {
                packets_.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
            }
        }

        packets.replace(packets_);
        collect.replace(collect_);
    }

    let elapsed = rv6.sys_rdtsc() - start;

    //println!("rx packets.len {} collect.len {} ", packets.len(), collect.len());
    //let ret = idev.device.submit_and_poll(&mut packets, &mut collect, false, false);
    //if collect_end && !collect.is_empty() {
        //let pkt = &collect[0];
        //dump_packet(pkt);
        //seq_end = BigEndian::read_u64(&pkt[42..42+8]);
    //}

    //println!("seq_start {} seq_end {} delta {}", seq_start, seq_end, seq_end - seq_start);
    println!("sum {} batch alloc_count {}", sum, alloc_count);
    println!("==> rx batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, sum, elapsed, elapsed / sum as u64);
    // dev.dump_stats();
    for hist in alloc::vec![submit_rx_hist, collect_rx_hist] {
        println!("hist:");
        // Iterate buckets that have observations
        for bucket in hist.iter().filter(|b| b.count > 0) {
            print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
            print!("\n");
        }
    }

    // println!("Reaped {} packets", dev.device.rx_poll_rref(poll.take().unwrap()).0);
}


// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("benchnet panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
