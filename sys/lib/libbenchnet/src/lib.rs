#![no_std]

extern crate alloc;
extern crate core;

mod maglev;
pub mod packettool;

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

macro_rules! print_hist {
    ($hist: ident) => {
        println!("{}", core::stringify!($hist));

        for bucket in $hist.iter().filter(|b| b.count > 0) {
            println!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
        }
    };
}
const BATCH_SIZE: usize = 32;

pub fn run_tx_udptest_rref(net: &dyn Net, pkt_len: usize, mut debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let batch_sz: usize = BATCH_SIZE;
    let mut packets = RRefDeque::<[u8; 1512], 32>::default();
    let mut collect = RRefDeque::<[u8; 1512], 32>::default();
    let mut poll =  RRefDeque::<[u8; 1512], 512>::default();

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

    let mut payload = alloc::vec![0u8; pkt_len];

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

    pkt_arr.copy_from_slice(pkt.as_slice());

    for i in 0..batch_sz {
        packets.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
    }

    let mut append_rdtsc: u64 = 0;
    let mut alloc_count = 0;
    let mut alloc_elapsed = 0;

    let mut packets = Some(packets);
    let mut collect = Some(collect);
    let mut poll = Some(poll);

    let mut collect_tx_hist = Base2Histogram::new();

    let mut sum: usize = 0;

    println!("======== Starting udp transmit test (rrefs)  ==========");

    let runtime = 30;

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();

    let end = rdtsc() + runtime * 2_600_000_000;

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
                packets_.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
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

    let adj_runtime = elapsed as f64 / 2_600_000_000_u64 as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("tx_udptest_rref ({}): Transmitted {} packets took {} cycles (avg = {})",
                                        pkt_len, sum, elapsed, elapsed as f64 / sum as f64);
        println!("Device Stats\n{}", stats_end);
        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else { 
        println!("Test failed! No packets transmitted");
    }

    //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
    let (done, mut poll_) = net.poll_rref(poll.take().unwrap(), true).unwrap()?;

    println!("Reaped {} packets", done);

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

    println!("======== Starting udp transmit test  ==========");

    let runtime = 30;

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = rdtsc() + runtime * 2_600_000_000;

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

    let adj_runtime = elapsed as f64 / 2_600_000_000_u64 as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("tx_udptest ({}): Transmitted {} packets took {} cycles (avg = {})",
                                        pkt_len, sum, elapsed, elapsed as f64 / sum as f64);

        println!("Device Stats\n{}", stats_end);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else { 
        println!("Test failed! No packets transmitted");
    }

    //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
    let done = net.poll(&mut collect, true).unwrap()?;

    println!("Reaped {} packets", done);

    print_hist!(collect_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}


pub fn run_rx_udptest_rref(net: &dyn Net, pkt_len: usize, debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    run_rx_udptest_rref_with_delay(net, pkt_len, debug, 0)
}

pub fn run_rx_udptest_rref_with_delay(net: &dyn Net, pkt_len: usize, debug: bool, delay: usize) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let pkt_len = 2048;
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

    println!("======== Starting udp rx test (rrefs) loop_delay: {} ==========", delay);

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
    let end = start + runtime * 2_600_000_000;

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
            packets_.push_back(packet);
        }

        if packets_.len() < batch_sz / 4 {
            //println!("allocating new batch");
            let alloc_sz = batch_sz - packets_.len();

            alloc_count += alloc_sz;

            let alloc_rdstc_start = rdtsc();
            for i in 0..alloc_sz {
                packets_.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
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


    let adj_runtime = elapsed as f64 / 2_600_000_000_u64 as f64;

    if sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);
        println!("rx_udptest_rref(delay: {}ns) : Received {} packets took {} cycles (avg = {})",
                                        delay, sum, elapsed, elapsed as f64 / sum as f64);

        println!("Device Stats\n{}", stats_end);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count, alloc_elapsed,
                                                        alloc_elapsed as f64 / alloc_count as f64);
    } else { 
        println!("Test failed! No packets Received");
    }

    let (done, mut poll_) = net.poll_rref(poll.take().unwrap(), false).unwrap()?;

    println!("Reaped {} packets", done);

    print_hist!(submit_rx_hist);

    print_hist!(collect_rx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}

pub fn run_rx_udptest(net: &dyn Net, pkt_len: usize, debug: bool) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let pkt_len = 2048;
    let batch_sz: usize = BATCH_SIZE;
    let mut packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
    let mut collect: VecDeque<Vec<u8>> = VecDeque::new();

    for i in 0..batch_sz {
        packets.push_front(Vec::with_capacity(pkt_len));
    }

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
        let ret = net.submit_and_poll(&mut packets, &mut collect, false).unwrap()?;
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
                packets.push_front(Vec::with_capacity(pkt_len));
            }
        }
    }

    let elapsed = rdtsc() - start;

    println!("rx packets.len {} collect.len {} ", packets.len(), collect.len());
    let ret = net.submit_and_poll(&mut packets, &mut collect, false);
    if collect_end && !collect.is_empty() {
        let pkt = &collect[0];
        dump_packet(pkt);
        seq_end = BigEndian::read_u64(&pkt[42..42+8]);
    }

    println!("seq_start {} seq_end {} delta {}", seq_start, seq_end, seq_end - seq_start);
    println!("sum {} batch alloc_count {}", sum, alloc_count);
    println!("==> rx batch {}B: {} iterations took {} cycles (avg = {})", pkt_len, sum, elapsed, elapsed / core::cmp::max(sum as u64, 1));
    // dev.dump_stats();
    for hist in alloc::vec![submit_rx_hist, collect_rx_hist] {
        println!("hist:");
        // Iterate buckets that have observations
        for bucket in hist.iter().filter(|b| b.count > 0) {
            print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
            print!("\n");
        }
    }

    println!("Reaped {} packets", net.poll(&mut collect, false).unwrap()?);
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

pub fn dump_packet_rref(pkt: &[u8; 1512], len: usize) {
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

// pub fn run_sashstoretest(net: &dyn Net, pkt_size: u16) {
//     let batch_sz = BATCH_SIZE;
//     let mut rx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
//     let mut tx_packets: VecDeque<Vec<u8>> = VecDeque::with_capacity(batch_sz);
//     let mut submit_rx_hist = Base2Histogram::new();
//     let mut submit_tx_hist = Base2Histogram::new();

//     for i in 0..batch_sz {
//         rx_packets.push_front(Vec::with_capacity(2048));
//     }

//     let mut sum: usize = 0;
//     let mut fwd_sum: usize = 0;

//     let start = rdtsc();
//     let end = start + 1200 * 2_600_000_000;

//     let mut tx_elapsed = 0;
//     let mut rx_elapsed = 0;

//     let mut submit_rx: usize = 0;
//     let mut submit_tx: usize = 0;
//     let mut loop_count: usize = 0;

//     loop {
//         loop_count = loop_count.wrapping_add(1);

//         submit_rx += rx_packets.len();
//         submit_rx_hist.record(rx_packets.len() as u64);
//         //println!("call rx_submit_poll packet {}", packets.len());
//         let rx_start = rdtsc();
//         let ret = net.submit_and_poll(&mut rx_packets, &mut tx_packets, false);
//         rx_elapsed += rdtsc() - rx_start;
//         sum += ret;

//         for mut pkt in tx_packets.iter_mut() {
//             if let Some((padding, payload)) = packettool::get_mut_udp_payload(pkt) {
//                 if let Some(mut sashstore) = unsafe { SASHSTORE.as_mut() } {
//                     let payloadptr = payload as *mut _ as *mut u8;
//                     let mut payloadvec = unsafe {
//                         Vec::from_raw_parts(
//                             payloadptr,
//                             payload.len(),
//                             2048 - padding, // FIXME: Awful
//                         )
//                     };

//                     // println!("Before handle: payloadvec.capacity() = {}, len() = {}", payloadvec.capacity(), payloadvec.len());
//                     let responsevec = unsafe { sashstore.handle_network_request(payloadvec) };

//                     // assert!(responsevec.as_ptr() == payloadptr);
//                     // println!("Handled: {:x?} -> {:x?}", responsevec.as_ptr(), payloadptr);
//                     // println!("After handle: responsevec.capacity() = {}, len() = {}", responsevec.capacity(), responsevec.len());
//                     if responsevec.as_ptr() != payloadptr {
//                         unsafe {
//                             ptr::copy(responsevec.as_ptr(), payloadptr, responsevec.len());
//                         }
//                     }

//                     // println!("Before set_len: {}", pkt.len());
//                     unsafe {
//                         pkt.set_len(padding + responsevec.len());
//                     }
//                     // println!("After set_len: padding={}, resposevec.len() = {}, set to {}", padding, responsevec.len(), pkt.len());

//                     packettool::swap_udp_ips(pkt);
//                     packettool::swap_mac(pkt);
//                     packettool::fix_ip_length(pkt);
//                     packettool::fix_ip_checksum(pkt);
//                     packettool::fix_udp_length(pkt);
//                     packettool::fix_udp_checksum(pkt);

//                     // println!("To send: {:x?}", pkt);
//                 } else {
//                     println!("No sashstore???");
//                 }
//             } else {
//                 // println!("Not a UDP packet: {:x?}", &pkt);
//             }
//         }

//         submit_tx += tx_packets.len();
//         submit_tx_hist.record(tx_packets.len() as u64);
//         let tx_start = rdtsc();
//         let ret = net.submit_and_poll(&mut tx_packets, &mut rx_packets, true);
//         tx_elapsed += rdtsc() - tx_start;
//         fwd_sum += ret;

//         //print!("tx: submitted {} collect {}\n", ret, rx_packets.len());

//         if rx_packets.len() == 0 && tx_packets.len() < batch_sz * 4 {
//             //println!("-> Allocating new rx_ptx batch");
//             for i in 0..batch_sz {
//                 rx_packets.push_front(Vec::with_capacity(2048));
//             }
//         }

//         if rdtsc() > end {
//             break;
//         }
//     }

//     let elapsed = rdtsc() - start;
//     for hist in alloc::vec![submit_rx_hist, submit_tx_hist] {
//         println!("hist:");
//         // Iterate buckets that have observations
//         for bucket in hist.iter().filter(|b| b.count > 0) {
//             print!("({:5}, {:5}): {}", bucket.start, bucket.end, bucket.count);
//             print!("\n");
//         }
//     }

//     println!("Received {} forwarded {}", sum, fwd_sum);
//     println!(" ==> submit_rx {} (avg {}) submit_tx {} (avg {}) loop_count {}",
//                         submit_rx, submit_rx / loop_count, submit_tx, submit_tx / loop_count, loop_count);
//     println!(" ==> rx batching {}B: {} packets took {} cycles (avg = {})",
//                         pkt_size, sum, rx_elapsed, rx_elapsed  / core::cmp::max(sum as u64, 1));
//     println!(" ==> tx batching {}B: {} packets took {} cycles (avg = {})",
//                         pkt_size, fwd_sum, tx_elapsed, tx_elapsed   / core::cmp::max(fwd_sum as u64, 1));
//     println!("==> fwd batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, fwd_sum, elapsed, elapsed  / core::cmp::max(fwd_sum as u64, 1));
//     // dev.dump_stats();
//     //dev.dump_tx_descs();
// }

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
        rx_packets.push_front(Vec::with_capacity(2048));
    }

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
        let ret = net.submit_and_poll(&mut rx_packets, &mut tx_packets, false).unwrap()?;
        rx_elapsed += rdtsc() - rx_start;
        sum += ret;

        //println!("rx: submitted {} collect {}", ret, tx_packets.len());

        for pkt in tx_packets.iter_mut() {
            let backend = {
                if let Some(hash) = packettool::get_flowhash(&pkt) {
                    Some(maglev.get_index_from_hash(hash))
                } else {
                    None
                }
            };

            if let Some(_) = backend {
                unsafe {
                    ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                    ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
                }
            }
        }

        submit_tx += tx_packets.len();
        submit_tx_hist.record(tx_packets.len() as u64);
        let tx_start = rdtsc();
        let ret = net.submit_and_poll(&mut tx_packets, &mut rx_packets, true).unwrap()?;
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
                        pkt_size, sum, rx_elapsed, rx_elapsed  / core::cmp::max(sum as u64, 1));
    println!(" ==> tx batching {}B: {} packets took {} cycles (avg = {})",
                        pkt_size, fwd_sum, tx_elapsed, tx_elapsed   / core::cmp::max(fwd_sum as u64, 1));
    println!("==> maglev fwd batch {}B: {} iterations took {} cycles (avg = {})", pkt_size, fwd_sum, elapsed, elapsed  / core::cmp::max(fwd_sum as u64, 1));
    // dev.dump_stats();
    //dev.dump_tx_descs();
    Ok(())
}

pub fn run_fwd_udptest_rref(net: &dyn Net, pkt_len: usize) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let batch_sz = BATCH_SIZE;
    let mut rx_submit = RRefDeque::<[u8; 1512], 32>::default();
    let mut rx_collect = RRefDeque::<[u8; 1512], 32>::default();
    let mut tx_poll =  RRefDeque::<[u8; 1512], 512>::default();
    let mut rx_poll =  RRefDeque::<[u8; 1512], 512>::default();

    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();
    
    let mut pkt_arr = [0; 1512];

    for i in 0..batch_sz {
        rx_submit.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
    }


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
    let end = start + runtime * 2_600_000_000;

    loop {
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let (ret, mut rx_submit_, mut rx_collect_) = net.submit_and_poll_rref(rx_submit.take().unwrap(),
                                rx_collect.take().unwrap(), false, pkt_len).unwrap()?;
        sum += ret;
        rx_elapsed += rdtsc() - rx_start;

        //println!("rx: submitted {} collect {}", ret, rx_collect_.len());

        let ms_start = rdtsc();
        for pkt in rx_collect_.iter_mut() {
            for i in 0..6 {
                (pkt).swap(i, 6 + i);
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
                rx_submit_.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
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

    let adj_runtime = elapsed as f64 / 2_600_000_000_u64 as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("fwd_udptest: Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);
        println!("Device Stats\n{}", stats_end);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    }

    let (done_rx, mut rx_poll_) = net.poll_rref(rx_poll.take().unwrap(), false).unwrap()?;
    let (done_tx, mut tx_poll_) = net.poll_rref(tx_poll.take().unwrap(), true).unwrap()?;

    println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

    print_hist!(submit_rx_hist);

    print_hist!(submit_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}

pub fn run_maglev_fwd_udptest_rref(net: &dyn Net, pkt_len: usize) -> Result<()> {
    #[cfg(feature = "noop")]
    return Ok(());

    let batch_sz = BATCH_SIZE;
    let mut maglev = maglev::Maglev::new(0..3);
    let mut rx_submit = RRefDeque::<[u8; 1512], 32>::default();
    let mut rx_collect = RRefDeque::<[u8; 1512], 32>::default();
    let mut tx_poll =  RRefDeque::<[u8; 1512], 512>::default();
    let mut rx_poll =  RRefDeque::<[u8; 1512], 512>::default();

    let mut submit_rx_hist = Base2Histogram::new();
    let mut submit_tx_hist = Base2Histogram::new();
    
    let mut pkt_arr = [0; 1512];

    for i in 0..batch_sz {
        rx_submit.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
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
    let end = start + runtime * 2_600_000_000;

    loop {
        //println!("call rx_submit_poll packet {}", packets.len());
        let rx_start = rdtsc();
        let (ret, mut rx_submit_, mut rx_collect_) = net.submit_and_poll_rref(rx_submit.take().unwrap(),
                                rx_collect.take().unwrap(), false, pkt_len).unwrap()?;
        sum += ret;
        rx_elapsed += rdtsc() - rx_start;

        //println!("rx: submitted {} collect {}", ret, rx_collect_.len());

        let ms_start = rdtsc();
        for pkt in rx_collect_.iter_mut() {
            let backend = {
                if let Some(hash) = packettool::get_flowhash(pkt) {
                    Some(maglev.get_index(&hash))
                } else {
                    None
                }
            };

            if let Some(_) = backend {
                for i in 0..6 {
                    (pkt).swap(i, 6 + i);
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
                rx_submit_.push_back(RRef::<[u8; 1512]>::new(pkt_arr.clone()));
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

    let adj_runtime = elapsed as f64 / 2_600_000_000_u64 as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("magleve_fwd_udptest: Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);
        println!("Device Stats\n{}", stats_end);

        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    }

    let (done_rx, mut rx_poll_) = net.poll_rref(rx_poll.take().unwrap(), false).unwrap()?;
    let (done_tx, mut tx_poll_) = net.poll_rref(tx_poll.take().unwrap(), true).unwrap()?;

    println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

    print_hist!(submit_rx_hist);

    print_hist!(submit_tx_hist);

    println!("+++++++++++++++++++++++++++++++++++++++++++++++++");
    Ok(())
}

pub fn run_fwd_udptest(net: &dyn Net, pkt_len: u16) -> Result<()> {
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

    println!("======== Starting udp fwd test ==========");

    let stats_start = net.get_stats().unwrap()?;

    let start = rdtsc();
    let end = start + runtime * 2_600_000_000;

    loop {
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
            unsafe {
                ptr::copy(our_mac.as_ptr(), pkt.as_mut_ptr().offset(6), our_mac.capacity());
                ptr::copy(sender_mac.as_ptr(), pkt.as_mut_ptr().offset(0), sender_mac.capacity());
            }
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

    let adj_runtime = elapsed as f64 / 2_600_000_000_u64 as f64;

    if sum > 0 && fwd_sum > 0 {
        println!("runtime: {:.2} seconds", adj_runtime);

        println!("Received packets: {} forwarded packets: {}", sum, fwd_sum);

        println!("Rx: {} packets took {} cycles (avg = {})", sum, rx_elapsed, rx_elapsed as f64 / sum as f64);

        println!("mac_swap: {} packets took {} cycles (avg = {})", sum, mswap_elapsed, mswap_elapsed as f64 / sum as f64);

        println!("Tx: {} packets took {} cycles (avg = {})", fwd_sum, tx_elapsed, tx_elapsed  as f64 / fwd_sum as f64);

        println!("fwd_udptest: Forwarding {} packets took {} cycles (avg = {})", fwd_sum, elapsed,
                                                                        elapsed as f64 / fwd_sum as f64);
        println!("Device Stats\n{}", stats_end);
        println!("Number of new allocations {}, took {} cycles (avg = {})", alloc_count * batch_sz, alloc_elapsed,
                                                        alloc_elapsed as f64 / (alloc_count * batch_sz) as f64);
    } else { 
        println!("Test failed! No packets Forwarded! Rxed {}, Txed {}", sum, fwd_sum);
    }

    //println!("packet.len {} collect.len {}", packets.unwrap().len(), collect.unwrap().len());
    let done_rx = net.poll(&mut rx_packets, false).unwrap()?;
    let done_tx = net.poll(&mut tx_packets, true).unwrap()?;

    println!("Reaped rx {} packets tx {} packets", done_rx, done_tx);

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
