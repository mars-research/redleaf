#![no_std]
#![feature(slice_fill,
            core_intrinsics
           )] // for vec::fill

extern crate alloc;
extern crate core;
extern crate sashstore_redleaf;

mod maglev;
pub mod packettool;

use core::alloc::Layout;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use b2histogram::Base2Histogram;
use byteorder::{BigEndian, ByteOrder};
use console::{print, println};
use core::ptr;
use libtime::get_rdtsc as rdtsc;
use libtime::sys_ns_loopsleep;
use packettool::{ETH_HEADER_LEN, IPV4_PROTO_OFFSET};
use rref::{RRef, RRefDeque};
use usr::net::{Net, NetworkStats};
use usr::error::Result;
use sashstore_redleaf::SashStore;

macro_rules! print_hist {
    ($hist: ident) => {
        println!("{}", core::stringify!($hist));

        for bucket in $hist.iter().filter(|b| b.count > 0) {
            println!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
        }
    };
}

const BATCH_SIZE: usize = 32;
const CPU_MHZ: u64 = 2_600_000_000;

pub fn run_tx_udptest_rref(net: &dyn Net, pkt_len: usize, mut debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let batch_sz: usize = BATCH_SIZE;
    let mut packets = RRefDeque::<[u8; 1514], 32>::default();
    let mut collect = RRefDeque::<[u8; 1514], 32>::default();
    let mut poll =  RRefDeque::<[u8; 1514], 512>::default();

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

    let mut payload = alloc::vec![0u8; pkt_len - 42];

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
    if len < 1514 {
        let pad = alloc::vec![0u8; 1514 - len];
        pkt.extend(pad.iter());
    }

    let mut pkt_arr = [0; 1514];

    pkt_arr.copy_from_slice(pkt.as_slice());

    for i in 0..batch_sz {
        packets.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
    }

    let mut append_rdtsc: u64 = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut packets = Some(packets);
    let mut collect = Some(collect);
    let mut poll = Some(poll);

    let mut collect_tx_hist = Base2Histogram::new();

    let mut sum: usize = 0;

    println!("======== Starting udp transmit test {}B (rrefs)  ==========", pkt_len);

    let runtime = 30;

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();

    let end = rdtsc() + runtime * CPU_MHZ;

    loop {
        let (ret, mut packets_, mut collect_) = net.submit_and_poll_rref(packets.take().unwrap(),
                                collect.take().unwrap(), true, pkt_len).unwrap()?;
        sum += ret;

        collect_tx_hist.record(collect_.len() as u64);

        while let Some(packet) = collect_.pop_front() {
            packets_.push_back(packet);
        }


        if packets_.len() == 0 {
            alloc_count += 1;
            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                packets_.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        //println!("packets_.len() {} collect_.len() {}", packets_.len(), collect_.len());

        packets.replace(packets_);
        collect.replace(collect_);

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("tx_udptest_rref ({}): Transmitted {} packets took {} cycles (avg = {})",
                                        pkt_len, sum, elapsed, elapsed as f64 / sum as f64);

        println!("Observed Pkts/s: {}", sum as f64 / adj_runtime as f64);

        //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
        let (done, mut poll_) = net.poll_rref(poll.take().unwrap(), true).unwrap()?;

        println!("Reaped {} packets", done);

        println!("Device Stats\n{}", stats_end);

        println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets transmitted");
    }

    print_hist!(collect_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}

pub fn run_tx_udptest(net: &dyn Net, pkt_len: usize, mut debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

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

    // pkt_len - header_size
    let mut payload = alloc::vec![0u8; pkt_len - 42];

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

    for i in 0..batch_sz {
        packets.push_front(pkt.clone());
    }

    let mut append_rdtsc: u64 = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut sum: usize = 0;
    let mut collect_tx_hist = Base2Histogram::new();

    println!("======== Starting udp transmit test {}B ==========", pkt_len);

    let runtime = 30;

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = rdtsc() + runtime * CPU_MHZ;

    loop{
        let ret = net.submit_and_poll(&mut packets, &mut collect, true).unwrap()?;

        sum += ret;

        collect_tx_hist.record(collect.len() as u64);

        packets.append(&mut collect);

        if packets.len() == 0 {
            alloc_count += 1;

            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                packets.push_front(pkt.clone());
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("tx_udptest ({}): Transmitted {} packets took {} cycles (avg = {})",
                                        pkt_len, sum, elapsed, elapsed as f64 / sum as f64);

        println!("Observed Pkts/s: {}", sum as f64 / adj_runtime as f64);

        //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
        let done = net.poll(&mut collect, true).unwrap()?;

        println!("Reaped {} packets", done);

        println!("Device Stats\n{}", stats_end);

        println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets transmitted");
    }

    print_hist!(collect_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}


#[inline(always)]
pub fn run_rx_udptest_rref(net: &dyn Net, pkt_len: usize, debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    run_rx_udptest_rref_with_delay(net, pkt_len, debug, 0)
}

#[inline(always)]
pub fn run_rx_udptest_rref_with_delay(net: &dyn Net, pkt_len: usize, debug: bool, delay: usize) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let req_pkt_len = pkt_len;
    let pkt_len = 2048;
    let batch_sz: usize = BATCH_SIZE;
    let mut packets = RRefDeque::<[u8; 1514], 32>::default();
    let mut collect = RRefDeque::<[u8; 1514], 32>::default();
    let mut poll =  RRefDeque::<[u8; 1514], 512>::default();

    let mut pkt_arr = [0; 1514];

    for i in 0..batch_sz {
        packets.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
    }

    let mut packets = Some(packets);
    let mut collect = Some(collect);
    let mut poll = Some(poll);


// pkgten.hz 2590000000 wire_size 704 lk 10000000000 pps 14204545 cpp 182 tx_cycles 11648 tx_rate 100.000000
    // compute the number of bits on wire for this pkt_len

    let wire_size = (req_pkt_len + 24) * 8;
    let link_speed = 10_000_000_000_u64;
    let pps = link_speed as f64 / wire_size as f64;
    let cpp = CPU_MHZ as f64 / pps;
    let rx_cycles = (cpp as u64 * batch_sz as u64);

    println!("CPU_MHZ {} wire_size {} link_speed {} pps {} cpp {} rx_cycles {}",
                        CPU_MHZ, wire_size, link_speed, pps, cpp, rx_cycles);
    println!("======== Starting udp rx test {}B (rrefs) loop_delay: {} ==========", pkt_len, delay);

    let mut sum: usize = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut submit_rx_hist = Base2Histogram::new();
    let mut collect_rx_hist = Base2Histogram::new();

    let mut collect_start = true;
    let mut collect_end = false;
    let mut seq_start: u64 = 0;
    let mut seq_end: u64 = 0;
    let runtime = 30;

    let stats_start = net.get_stats().unwrap()?;
    let start = rdtsc();
    let end = start + runtime * CPU_MHZ;

    loop {
        // We observed that rx performance would slightly improve if we introduce this delay in
        // every loop. The hypothesis is that, some sort of pointer thrashing is happening between
        // the cpu and hardware. This delay was empirically found.
        sys_ns_loopsleep(delay as u64);

        let (ret, mut packets_, mut collect_) = net.submit_and_poll_rref(packets.take().unwrap(),
                                collect.take().unwrap(), false, pkt_len).unwrap()?;

        //if debug {
            //println!("rx packets.len {} collect.len {} ret {}", packets.len(), collect.len(), ret);
        //}
        sum += collect_.len();
        collect_rx_hist.record(collect_.len() as u64);


        while let Some(packet) = collect_.pop_front() {
            if collect_start {
                dump_packet_rref(&packet, 100);
                collect_start = false;
            }

            packets_.push_back(packet);
        }

        if (batch_sz == 1 && packets_.len() == 0) || (batch_sz > 1 && packets_.len() < batch_sz / 4) {
            //println!("allocating new batch");
            let alloc_sz = batch_sz - packets_.len();

            alloc_count += alloc_sz;

            let alloc_rdstc_start = rdtsc();
            for i in 0..alloc_sz {
                packets_.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        submit_rx_hist.record(packets_.len() as u64);

        packets.replace(packets_);
        collect.replace(collect_);

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("rx_udptest_rref(delay: {}ns) : Received {} packets took {} cycles (avg = {})",
                                        delay, sum, elapsed, elapsed as f64 / sum as f64);

        let (done, mut poll_) = net.poll_rref(poll.take().unwrap(), false).unwrap()?;

        println!("Reaped {} packets", done);

        println!("Device Stats\n{}", stats_end);

        println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count, alloc_elapsed,
                                                        alloc_elapsed as f64 / alloc_count as f64);
    } else {
        println!("Test failed! No packets Received");
    }

    print_hist!(submit_rx_hist);
    print_hist!(collect_rx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}

pub fn run_rx_udptest(net: &dyn Net, pkt_len: usize, debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    run_rx_udptest_with_delay(net, pkt_len, debug, 0)
}

pub fn run_rx_udptest_with_delay(net: &dyn Net, pkt_len: usize, debug: bool, delay: usize) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let req_pkt_len = pkt_len;
    let pkt_len = 2048;
    let batch_sz: usize = BATCH_SIZE;
    let mut packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut collect: VecDeque<Vec<u8>> = VecDeque::new();

    for i in 0..batch_sz {
        packets.push_front(Vec::with_capacity(pkt_len));
    }

    let mut sum: usize = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut submit_rx_hist = Base2Histogram::new();
    let mut collect_rx_hist = Base2Histogram::new();

    let mut collect_start = true;
    let mut collect_end = false;
    let mut seq_start: u64 = 0;
    let mut seq_end: u64 = 0;
    let runtime = 30;

    println!("======== Starting udp rx test {}B loop_delay: {} ==========", pkt_len, delay);

    let stats_start = net.get_stats().unwrap()?;
    let start = rdtsc();
    let end = start + runtime * CPU_MHZ;

    loop {
        // We observed that rx performance would slightly improve if we introduce this delay in
        // every loop. The hypothesis is that, some sort of pointer thrashing is happening between
        // the cpu and hardware. This delay was empirically found.
        sys_ns_loopsleep(delay as u64);

        submit_rx_hist.record(packets.len() as u64);

        let ret = net.submit_and_poll(&mut packets, &mut collect, false).unwrap()?;

        if debug {
            println!("rx packets.len {} collect.len {} ret {}", packets.len(), collect.len(), ret);
        }

        sum += collect.len();
        collect_rx_hist.record(collect.len() as u64);

        /*if collect_start && !collect.is_empty() {
            let pkt = &collect[0];
            dump_packet(pkt);
            seq_start = BigEndian::read_u64(&pkt[42..42+8]);
            collect_start = false;
            collect_end = true;
        }*/

        packets.append(&mut collect);
 
        if (batch_sz == 1 && packets.len() == 0) || (batch_sz > 1 && packets.len() < batch_sz / 4) {
            //println!("allocating new batch");
            alloc_count += 1;

            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                packets.push_front(Vec::with_capacity(pkt_len));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        if rdtsc() > end {
            break;
        }

    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("rx_udptest (delay: {}ns) : Received {} packets took {} cycles (avg = {})",
                                        delay, sum, elapsed, elapsed as f64 / sum as f64);

        let done = net.poll(&mut collect, false).unwrap()?;

        println!("Reaped {} packets", done);

        println!("Device Stats\n{}", stats_end);

        println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count, alloc_elapsed,
                                                        alloc_elapsed as f64 / alloc_count as f64);
    } else {
        println!("Test failed! No packets Received");
    }

    print_hist!(submit_rx_hist);
    print_hist!(collect_rx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");

    Ok(())
}

pub fn dump_packet(pkt: &Vec<u8>) {
    for (i, b) in pkt.iter().enumerate() {
        print!("{:02X} ", b); 

        if i > 0 && (i + 1) % 25 == 0 { 
            print!("\n");
        }
    }
    print!("\n");
}

pub fn dump_packet_rref(pkt: &[u8; 1514], len: usize) {
    for (i, b) in pkt.iter().enumerate() {
        print!("{:02X} ", b); 

        if i > 0 && (i + 1) % 25 == 0 { 
            print!("\n");
        }
        if i >= len {
            break;
        }
    }
    print!("\n");
}

static mut SASHSTORE: Option<SashStore> = None;

pub fn run_sashstoretest(net: &dyn Net, pkt_size: u16) -> Result<()> {
    let batch_sz = BATCH_SIZE;
    let mut rx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut tx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();

    unsafe {
        // SASHSTORE = Some(SashStore::with_capacity((1 << 20)));
        SASHSTORE = Some(SashStore::with_capacity(1 << 21));
    }


    for i in 0..batch_sz {
        rx_packets.push_front(Vec::with_capacity(2048));
    }

    let mut sum: usize = 0;
    let mut fwd_sum: usize = 0;

    println!("======== Starting sashstore test ==========");
    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = start + 60 * CPU_MHZ;

    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut tx_elapsed = 0;
    let mut rx_elapsed = 0;
    let mut lookup_elapsed = 0;

    let mut submit_rx: usize = 0;
    let mut submit_tx: usize = 0;
    let mut loop_count: usize = 0;

    loop {
        loop_count = loop_count.wrapping_add(1);

        submit_rx += rx_packets.len();
        submit_rx_hist.record(rx_packets.len() as u64);
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let ret = net.submit_and_poll(&mut rx_packets, &mut tx_packets, false).unwrap()?;
        rx_elapsed += rdtsc() - rx_start;
        sum += ret;

        let lookup_start = rdtsc();

        for i in 0..tx_packets.len() {
            // Prefetch ahead
            {
                if i < (tx_packets.len() - 1) {
                    let pkt_next = &tx_packets[i + 1];
                    unsafe {
                        core::intrinsics::prefetch_write_data(pkt_next.as_ptr(), 3);
                        core::intrinsics::prefetch_write_data(pkt_next.as_ptr().offset(64), 3);
                        core::intrinsics::prefetch_write_data(pkt_next.as_ptr().offset(128), 3);
                    }
                }
            }

            let mut pkt = &mut tx_packets[i];

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
                        println!("copied");
                    }

                    // println!("Before set_len: {}", pkt.len());
                    unsafe {
                        pkt.set_len(padding + responsevec.len());
                    }
                    // println!("After set_len: padding={}, resposevec.len() = {}, set to {}", padding, responsevec.len(), pkt.len());

                    //packettool::swap_udp_ips(pkt);
                    //packettool::swap_mac(pkt);
                    //packettool::fix_ip_length(pkt);
                    //packettool::fix_ip_checksum(pkt);
                    //packettool::fix_udp_length(pkt);
                    //packettool::fix_udp_checksum(pkt);

                    // println!("To send: {:x?}", pkt);
                } else {
                    println!("No sashstore???");
                }
            } else {
                // println!("Not a UDP packet: {:x?}", &pkt);
            }
        }

        lookup_elapsed += rdtsc() - lookup_start;

        submit_tx += tx_packets.len();
        submit_tx_hist.record(tx_packets.len() as u64);
        let tx_start = rdtsc();
        let ret = net.submit_and_poll(&mut tx_packets, &mut rx_packets, true).unwrap()?;
        tx_elapsed += rdtsc() - tx_start;
        fwd_sum += ret;

        //print!("tx: submitted {} collect {}\n", ret, rx_packets.len());

        if rx_packets.len() == 0 && tx_packets.len() < batch_sz * 4 {
            //println!("-> Allocating new rx_ptx batch");
            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                rx_packets.push_front(Vec::with_capacity(2048));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("KV lookup: {} packets took {} cycles (avg = {})", sum, lookup_elapsed, lookup_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("Sashstore: {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);

        let done_rx = net.poll(&mut rx_packets, false).unwrap()?;
        let done_tx = net.poll(&mut tx_packets, true).unwrap()?;

        println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

        println!("Device Stats\n{}", stats_end);

        println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);
        println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets Forwarded! Rxed {}, Txed {}", sum, fwd_sum);
    }

    print_hist!(submit_rx_hist);

    print_hist!(submit_tx_hist);

    if let Some(mut sashstore) = unsafe { SASHSTORE.as_mut() } {
        sashstore.print_stats();
    }
    Ok(())
}

pub fn run_fwd_maglevtest(net: &dyn Net, pkt_size: u16) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let batch_sz = BATCH_SIZE;
    let mut maglev = maglev::Maglev::new(0..3);
    let mut rx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut tx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();
    
    let mut sender_mac = alloc::vec![ 0x90, 0xe2, 0xba, 0xb3, 0x74, 0x81];
    let mut our_mac = alloc::vec![0x90, 0xe2, 0xba, 0xb5, 0x14, 0xcd];


    for i in 0..batch_sz {
        // Attempt to make the buffers align at different offsets - to avoid eviction from L1
        /*let mut vec = unsafe {
            let layout = Layout::from_size_align(4096, 4096)
                    .map_err(|e| panic!("Layout error: {}", e)).unwrap();

            let buf = unsafe {alloc::alloc::alloc(layout) as *mut u8 };
            let mut v: Vec<u8> = unsafe { Vec::from_raw_parts(buf.offset(64 * i as isize), 64, 64) };
            v
        };
        rx_packets.push_front(vec);
        */
        rx_packets.push_front(Vec::with_capacity(2048));
    }

    /*for i in 0..batch_sz {
        println!("buf_addr[{}] = {:x}", i, rx_packets[i].as_ptr() as *const _ as *const u64 as u64);
    }*/

    let mut sum: usize = 0;
    let mut fwd_sum: usize = 0;

    println!("======== Starting maglev test ==========");
    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = start + 30 * CPU_MHZ;

    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut mswap_elapsed = 0;

    let mut tx_elapsed = 0;
    let mut rx_elapsed = 0;

    let mut submit_rx: usize = 0;
    let mut submit_tx: usize = 0;
    let mut loop_count: usize = 0;
    let mut hash_sum: usize = 0;

    let delay = 350;
    loop {

       // sys_ns_loopsleep(delay as u64);

        loop_count = loop_count.wrapping_add(1);

        submit_rx += rx_packets.len();
        submit_rx_hist.record(rx_packets.len() as u64);
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let ret = net.submit_and_poll(&mut rx_packets, &mut tx_packets, false).unwrap()?;
        rx_elapsed += rdtsc() - rx_start;
        sum += ret;

        //println!("rx: submitted {} collect {}", ret, tx_packets.len());

        let ms_start = rdtsc();

        for i in 0..tx_packets.len() {
            // Prefetch ahead
            {
                if i < (tx_packets.len() - 2) {
                    if ((i + 1) < tx_packets.len()) && ((i + 2) < tx_packets.len()) {
                        let pkt_next = &tx_packets[i + 1];
                        let pkt_next2 = &tx_packets[i + 2];
                        unsafe {
                            core::intrinsics::prefetch_write_data(pkt_next.as_ptr(), 3);
                            // core::arch::x86_64::_mm_prefetch(pkt_next.as_ptr() as *const i8, 3);
                            core::intrinsics::prefetch_write_data(pkt_next2.as_ptr(), 3);
                            // core::arch::x86_64::_mm_prefetch(pkt_next2.as_ptr() as *const i8, 3);
                        }
                    }
                }
            }

            let mut pkt = &mut tx_packets[i];
            let backend = {
                if let Some(hash) = packettool::get_flowhash(&pkt) {
                    Some(maglev.get_index_from_hash(hash))
                } else {
                    None
                }
            };

            if let Some(b) = backend {
                unsafe { 
                    ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                    ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
                }
            };

            //hash_sum = hash_sum.wrapping_add(backend.unwrap());
        }

        mswap_elapsed += rdtsc() - ms_start;

        submit_tx += tx_packets.len();
        submit_tx_hist.record(tx_packets.len() as u64);
        let tx_start = rdtsc();
        let ret = net.submit_and_poll(&mut tx_packets, &mut rx_packets, true).unwrap()?;
        tx_elapsed += rdtsc() - tx_start;
        fwd_sum += ret;

        //print!("tx: submitted {} collect {}\n", ret, rx_packets.len());

        if rx_packets.len() == 0 && tx_packets.len() < batch_sz * 4 {
            //println!("-> Allocating new rx_ptx batch");
            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                rx_packets.push_front(Vec::with_capacity(2048));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("maglev_fwd : Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);

        let done_rx = net.poll(&mut rx_packets, false).unwrap()?;
        let done_tx = net.poll(&mut tx_packets, true).unwrap()?;

        println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

        println!("Device Stats\n{}", stats_end);

        println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);
        println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets Forwarded! Rxed {}, Txed {}", sum, fwd_sum);
    }

    print_hist!(submit_rx_hist);

    print_hist!(submit_tx_hist);

    maglev.dump_stats();

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");

    Ok(())
}

pub fn run_fwd_udptest_rref(net: &dyn Net, pkt_len: usize) -> Result<()> {
    run_fwd_udptest_rref_with_delay(net, pkt_len, 0)
}

pub fn run_fwd_udptest_rref_with_delay(net: &dyn Net, pkt_len: usize, delay: u64) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let batch_sz = BATCH_SIZE;
    let mut rx_submit = RRefDeque::<[u8; 1514], 32>::default();
    let mut rx_collect = RRefDeque::<[u8; 1514], 32>::default();
    let mut tx_poll =  RRefDeque::<[u8; 1514], 512>::default();
    let mut rx_poll =  RRefDeque::<[u8; 1514], 512>::default();

    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();
    
    let mut pkt_arr = [0; 1514];

    for i in 0..batch_sz {
        rx_submit.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
    }


    let mut sender_mac = alloc::vec![ 0x90, 0xe2, 0xba, 0xb3, 0x74, 0x81];
    let mut our_mac = alloc::vec![0x90, 0xe2, 0xba, 0xb5, 0x14, 0xcd];

    let mut sum: usize = 0;
    let mut fwd_sum: usize = 0;

    let mut append_rdtsc: u64 = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut tx_elapsed = 0;
    let mut rx_elapsed = 0;

    let mut mswap_elapsed = 0;

    let mut submit_rx: usize = 0;
    let mut submit_tx: usize = 0;

    let mut rx_submit = Some(rx_submit);
    let mut rx_collect = Some(rx_collect);

    let mut tx_poll = Some(tx_poll);
    let mut rx_poll = Some(rx_poll);
    let runtime = 30;

    println!("======== Starting udp fwd test (rrefs) ==========");

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = start + runtime * CPU_MHZ;

    loop {
        sys_ns_loopsleep(delay);
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let (ret, mut rx_submit_, mut rx_collect_) = net.submit_and_poll_rref(rx_submit.take().unwrap(),
                                rx_collect.take().unwrap(), false, pkt_len).unwrap()?;
        sum += ret;
        rx_elapsed += rdtsc() - rx_start;

        //println!("rx: submitted {} collect {}", ret, rx_collect_.len());

        let ms_start = rdtsc();
        for pkt in rx_collect_.iter_mut() {
            /*for i in 0..6 {
                (pkt).swap(i, 6 + i);
            }*/
            //let mut pkt = pkt as *mut [u8; 1514] as *mut u8;
            unsafe {
                ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
            }
        }
        mswap_elapsed += rdtsc() - ms_start;

        submit_tx += rx_collect_.len();
        submit_tx_hist.record(rx_collect_.len() as u64);

        let tx_start = rdtsc();
        let (ret, mut rx_collect_, mut rx_submit_) = net.submit_and_poll_rref(rx_collect_,
                                rx_submit_, true, pkt_len).unwrap()?;

        tx_elapsed += rdtsc() - tx_start;
        fwd_sum += ret;

        //print!("tx: submitted {} collect {}\n", ret, tx_collect_.len());

        if rx_submit_.len() == 0 && rx_collect_.len() < batch_sz * 4 {
            //println!("-> Allocating new rx_ptx batch");
            alloc_count += 1;

            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                rx_submit_.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        submit_rx += rx_submit_.len();
        submit_rx_hist.record(rx_submit_.len() as u64);

        rx_submit.replace(rx_submit_);
        rx_collect.replace(rx_collect_);

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("fwd_udptest: Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);

        let (done_rx, mut rx_poll_) = net.poll_rref(rx_poll.take().unwrap(), false).unwrap()?;
        let (done_tx, mut tx_poll_) = net.poll_rref(tx_poll.take().unwrap(), true).unwrap()?;

        println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

        println!("Device Stats\n{}", stats_end);

        println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);
        println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets Forwarded! Rxed {}, Txed {}", sum, fwd_sum);
    }

    print_hist!(submit_rx_hist);
    print_hist!(submit_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}

pub fn run_maglev_fwd_udptest_rref(net: &dyn Net, pkt_len: usize) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let mut sender_mac = alloc::vec![ 0x90, 0xe2, 0xba, 0xb3, 0x74, 0x81];
    let mut our_mac = alloc::vec![0x90, 0xe2, 0xba, 0xb5, 0x14, 0xcd];

    let batch_sz = BATCH_SIZE;
    let mut maglev = maglev::Maglev::new(0..3);
    let mut rx_submit = RRefDeque::<[u8; 1514], 32>::default();
    let mut rx_collect = RRefDeque::<[u8; 1514], 32>::default();
    let mut tx_poll =  RRefDeque::<[u8; 1514], 512>::default();
    let mut rx_poll =  RRefDeque::<[u8; 1514], 512>::default();

    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();
    
    let mut pkt_arr = [0; 1514];

    for i in 0..batch_sz {
        rx_submit.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
    }

    // Make the packets valid so that they don't get rejected by maglev
    for packet in rx_submit.iter_mut() {
        // Set protocol to ipv4
        packet[ETH_HEADER_LEN] = 0b0100_0000;

        // Set protocol to TCP
        packet[ETH_HEADER_LEN + IPV4_PROTO_OFFSET] = 6;
    }


    let mut sum: usize = 0;
    let mut fwd_sum: usize = 0;

    let mut tx_elapsed = 0;
    let mut rx_elapsed = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut mswap_elapsed = 0;

    let mut submit_rx: usize = 0;
    let mut submit_tx: usize = 0;

    let mut rx_submit = Some(rx_submit);
    let mut rx_collect = Some(rx_collect);

    let mut tx_poll = Some(tx_poll);
    let mut rx_poll = Some(rx_poll);

    let runtime = 30;

    println!("======== Starting udp maglev fwd test (rrefs) ==========");

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = start + runtime * CPU_MHZ;

    loop {
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let (ret, mut rx_submit_, mut rx_collect_) = net.submit_and_poll_rref(rx_submit.take().unwrap(),
                                rx_collect.take().unwrap(), false, pkt_len).unwrap()?;
        sum += ret;
        rx_elapsed += rdtsc() - rx_start;

        //println!("rx: submitted {} collect {}", ret, rx_collect_.len());

        let ms_start = rdtsc();


        let mut pkt_iter = rx_collect_.iter_mut();

        while let Some(pkt) = pkt_iter.next() {
            let next = pkt_iter.next();
            // TODO : Prefetch the next packet here
            //
            let backend = {
                if let Some(hash) = packettool::get_flowhash(pkt) {
                    Some(maglev.get_index_from_hash(hash))
                } else {
                    None
                }
            };
 
            if let Some(_) = backend {
                /*
                for i in 0..6 {
                    (pkt).swap(i, 6 + i);
                }
                */
                unsafe {
                    ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                    ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
                }
            }
         }

        mswap_elapsed += rdtsc() - ms_start;

        submit_tx += rx_collect_.len();
        submit_tx_hist.record(rx_collect_.len() as u64);

        let tx_start = rdtsc();
        let (ret, mut rx_collect_, mut rx_submit_) = net.submit_and_poll_rref(rx_collect_,
                                rx_submit_, true, pkt_len).unwrap()?;

        tx_elapsed += rdtsc() - tx_start;
        fwd_sum += ret;

        //print!("tx: submitted {} collect {}\n", ret, tx_collect_.len());

        if rx_submit_.len() == 0 && rx_collect_.len() < batch_sz * 4 {
            //println!("-> Allocating new rx_ptx batch");
            alloc_count += 1;
            let alloc_rdstc_start = rdtsc();
            for i in 0..batch_sz {
                rx_submit_.push_back(RRef::<[u8; 1514]>::new(pkt_arr.clone()));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }


        submit_rx += rx_submit_.len();
        submit_rx_hist.record(rx_submit_.len() as u64);
        rx_submit.replace(rx_submit_);
        rx_collect.replace(rx_collect_);

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("magleve_fwd_udptest: Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);

        let (done_rx, mut rx_poll_) = net.poll_rref(rx_poll.take().unwrap(), false).unwrap()?;
        let (done_tx, mut tx_poll_) = net.poll_rref(tx_poll.take().unwrap(), true).unwrap()?;

        println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

        println!("Device Stats\n{}", stats_end);

        println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);
        println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets Forwarded! Rxed {}, Txed {}", sum, fwd_sum);
    }

    print_hist!(submit_rx_hist);

    print_hist!(submit_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    maglev.dump_stats();

    Ok(())
}

pub fn run_fwd_udptest(net: &dyn Net, pkt_len: u16) -> Result<()> {
    run_fwd_udptest_with_delay(net, pkt_len, 0)
}


pub fn run_fwd_udptest_with_delay(net: &dyn Net, pkt_len: u16, delay: u64) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

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

    let mut sum: usize = 0;
    let mut fwd_sum: usize = 0;

    let mut append_rdtsc: u64 = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut tx_elapsed = 0;
    let mut rx_elapsed = 0;

    let mut mswap_elapsed = 0;
    let mut bswap_elapsed = 0;

    let mut submit_rx: usize = 0;
    let mut submit_tx: usize = 0;
    let runtime = 30;

    println!("======== Starting udp fwd test (delay {} ns)==========", delay);

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = start + runtime * CPU_MHZ;

    loop {
        sys_ns_loopsleep(delay);
        submit_rx += rx_packets.len();
        submit_rx_hist.record(rx_packets.len() as u64);
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let ret = net.submit_and_poll(&mut rx_packets, &mut tx_packets, false).unwrap()?;
        rx_elapsed += rdtsc() - rx_start;
        sum += ret;

        //println!("rx: submitted {} collect {}", ret, tx_packets.len());

        let ms_start = rdtsc();
        for pkt in tx_packets.iter_mut() {

            /*for i in 0..6 {
                (pkt).swap(i, 6 + i);
            }*/
            /*unsafe {
                ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
            }*/
        }
        mswap_elapsed += rdtsc() - ms_start;

        submit_tx += tx_packets.len();
        submit_tx_hist.record(tx_packets.len() as u64);

        let tx_start = rdtsc();
        let ret = net.submit_and_poll(&mut tx_packets, &mut rx_packets, true).unwrap()?;
        tx_elapsed += rdtsc() - tx_start;
        fwd_sum += ret;

        //print!("tx: submitted {} collect {}\n", ret, rx_packets.len());

        if rx_packets.len() == 0 && tx_packets.len() < batch_sz * 4 {
            //println!("-> Allocating new rx_ptx batch");
            alloc_count += 1;

            let alloc_rdstc_start = rdtsc();

            for i in 0..batch_sz {
                rx_packets.push_front(Vec::with_capacity(2048));
            }
            alloc_elapsed += rdtsc() - alloc_rdstc_start;
        }

        if rdtsc() > end {
            break;
        }
    }

    let elapsed = rdtsc() - start;

    let mut stats_end = net.get_stats().unwrap()?;

    stats_end.stats_diff(stats_start);

    let adj_runtime = elapsed as f64 / CPU_MHZ as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("fwd_udptest: Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);

        //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
        let done_rx = net.poll(&mut rx_packets, false).unwrap()?;
        let done_tx = net.poll(&mut tx_packets, true).unwrap()?;

        println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else {
        println!("Test failed! No packets Forwarded! Rxed {}, Txed {}", sum, fwd_sum);
    }

    println!("Device Stats\n{}", stats_end);

    println!("Tx Pkts/s {:.2}", stats_end.tx_dma_ok as f64 / adj_runtime as f64);
    println!("Rx Pkts/s {:.2}", stats_end.rx_dma_ok as f64 / adj_runtime as f64);

    print_hist!(submit_rx_hist);
    print_hist!(submit_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");

    //dev.dump_tx_descs();
    Ok(())
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
