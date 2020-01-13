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
    panic_info_message
)]

extern crate malloc;
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use libsyscalls::syscalls::{sys_create_thread, sys_yield, sys_backtrace};
use console::println;
use protocol::{UdpPacket, PAYLOAD_SZ};
use spin::Mutex;
use alloc::sync::Arc;
use libsyscalls::time::sys_ns_sleep;

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

fn construct_udp_packet() -> Arc<Mutex<UdpPacket>> {
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
    Arc::new(Mutex::new(pkt))
}

#[no_mangle]
pub fn kvstore_init(s: Box<dyn syscalls::Syscall + Send + Sync>, net: Box<dyn syscalls::Net>)
{
    libsyscalls::syscalls::init(s);
    //let buf = [0, 1, 2];
    //net.send(&buf);
    let packet = construct_udp_packet();

    net.send_udp(packet.clone());

    sys_ns_sleep(1_000_000_000 * 10);

    println!("===> pushing out first batch of 20 million packets");

    for i in 0..20 {
        net.send_udp(packet.clone());
    }

    println!("===> Done pushing out first batch of 20 million packets");

    println!("===> Sleeping for 10seconds");

    sys_ns_sleep(1_000_000_000 * 10);

    println!("===> Woken up after sleep of 10seconds");

    println!("===> pushing out second batch of 20 million packets");

    for i in 0..20 {
        net.send_udp(packet.clone());
    }

    println!("===> Done pushing out second batch of 20 million packets");
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    sys_backtrace();
    loop {}
}
