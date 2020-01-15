#![no_std]
#[macro_use]
extern crate bitfield;

use core::convert::TryInto;

mod headers;

pub use crate::headers::eth::EthernetHeader;
pub use crate::headers::ipv4::IpV4Header;
pub use crate::headers::udp::UdpHeader;

pub const PAYLOAD_SZ: usize = 18;
pub const ETH_HDR_SZ: usize = 14;
pub const IP_HDR_SZ: usize = 20;
pub const UDP_HDR_SZ: usize = 8;
// TODO: Make this a generic
pub const MTU_SZ: usize = 60;

#[repr(C)]
#[derive(Debug)]
pub struct UdpPacket {
    pub eth_hdr: EthernetHeader<[u8; ETH_HDR_SZ]>,
    pub ip_hdr: IpV4Header<[u8; IP_HDR_SZ]>,
    pub udp_hdr: UdpHeader<[u8; UDP_HDR_SZ]>,
    pub payload: [u8; PAYLOAD_SZ],
}

use core::convert::AsMut;

fn copy_into_array(slice: &[u8]) -> [u8; PAYLOAD_SZ]
{
    let mut a = [0u8; PAYLOAD_SZ];
    unsafe {
        core::ptr::copy(slice.as_ptr(), a.as_mut() as *mut _ as *mut u8, PAYLOAD_SZ);
    }
    a
}

impl From<[u8; MTU_SZ]> for UdpPacket {
    fn from(buf: [u8; MTU_SZ]) -> UdpPacket {
        UdpPacket {
            eth_hdr: EthernetHeader(buf[0..ETH_HDR_SZ].try_into().expect("Could not convert")),
            ip_hdr: IpV4Header(buf[ETH_HDR_SZ..(ETH_HDR_SZ+IP_HDR_SZ)].try_into().expect("Could not convert")),
            udp_hdr: UdpHeader(buf[(ETH_HDR_SZ+IP_HDR_SZ)..(ETH_HDR_SZ+IP_HDR_SZ+UDP_HDR_SZ)].try_into().expect("Could not convert")),
            payload: copy_into_array(&buf[(ETH_HDR_SZ+IP_HDR_SZ+UDP_HDR_SZ)..]),
        }
    }
}

impl UdpPacket {
    pub fn new(eth_hdr: EthernetHeader<[u8; ETH_HDR_SZ]>,
               ip_hdr: IpV4Header<[u8; IP_HDR_SZ]>,
               udp_hdr: UdpHeader<[u8; UDP_HDR_SZ]>,
               payload: [u8; PAYLOAD_SZ]) -> UdpPacket {
        UdpPacket {
            eth_hdr,
            ip_hdr,
            udp_hdr,
            payload,
        }
    }

    pub fn new_raw(buf: [u8; MTU_SZ]) -> UdpPacket {
        UdpPacket::from(buf)
    }

    pub fn new_zeroed() -> UdpPacket {
        UdpPacket {
            eth_hdr: EthernetHeader([0u8; ETH_HDR_SZ]),
            ip_hdr: IpV4Header([0u8; IP_HDR_SZ]),
            udp_hdr: UdpHeader([0u8; UDP_HDR_SZ]),
            payload: [0u8; PAYLOAD_SZ],
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dump_headers() {
        use super::{EthernetHeader, IpV4Header, UdpHeader};

        let mac_data = [
            0xa0, 0x36, 0x9f, 0x08, 0x1f, 0x9e,
            0xa0, 0x36, 0x9f, 0x08, 0x1c, 0x4a,
            0x08, 0x00,
        ];

        let ip_data = [
            0x45, 0x00, 0x00, 0x1d, 0x6d, 0xc2, 0x40, 0x00,
            0x40, 0x11, 0x49, 0xba, 0xc0, 0xa8, 0x01, 0x01,
            0xc0, 0xa8, 0x01, 0x02,
        ];

        let udp_hdr = [
            0xdf, 0xb6, 0x14, 0x51, 0x00, 0x09, 0x7c, 0x80,
        ];

        let eth_header = EthernetHeader(mac_data);
        let ipv4_header = IpV4Header(ip_data);
        let udp_header = UdpHeader(udp_hdr);
        // TODO: Do assertions
        // To see println run it with
        // cargo test -- --nocapture
        println!("Ethernet header {:#?}", eth_header);
        println!("Ip header {:#x?}", ipv4_header);
        println!("Udp header {:#?}", udp_header);
    }
}
