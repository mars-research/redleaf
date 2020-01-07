#[macro_use]
extern crate bitfield;

mod headers;

pub use crate::headers::eth::EthernetHeader;
pub use crate::headers::ipv4::IpV4Header;
pub use crate::headers::udp::UdpHeader;

#[repr(C)]
pub struct UdpPacket<'p> {
    eth_hdr: EthernetHeader<[u8;14]>,
    ip_hdr: IpV4Header<[u8; 20]>,
    udp_hdr: UdpHeader<[u8; 8]>,
    payload: &'p [u8],
}

impl<'p> UdpPacket<'p> {
    pub fn new(eth_hdr: EthernetHeader<[u8; 14]>,
               ip_hdr: IpV4Header<[u8; 20]>,
               udp_hdr: UdpHeader<[u8; 8]>,
               payload: &'p [u8]) -> UdpPacket {
        UdpPacket {
            eth_hdr,
            ip_hdr,
            udp_hdr,
            payload,
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
