use core::fmt;

pub struct Ipv4Address {
    addr: [u8; 4],
}

impl From<[u8; 4]> for Ipv4Address {
    fn from(octets: [u8; 4]) -> Ipv4Address {
        let mut ip: Ipv4Address = Ipv4Address { addr: [0u8; 4] };
        for (i, octets) in octets.iter().enumerate() {
            ip.addr[i] = *octets;
        }
        ip
    }
}

impl From<u32> for Ipv4Address {
    fn from(ip: u32) -> Ipv4Address {
        Ipv4Address::from(ip.to_be_bytes())
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.addr[0], self.addr[1], self.addr[2], self.addr[3])
    }
}

impl fmt::Debug for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

bitfield! {
    pub struct IpV4Header(MSB0 [u8]);
    impl Debug;
    u32;
    get_version, set_version: 3, 0;
    get_ihl, set_ihl: 7, 4;
    get_dscp, set_dscp: 13, 8;
    get_ecn, set_ecn: 15, 14;
    get_total_length, set_total_length: 31, 16;
    get_identification, set_identification: 47, 32;
    get_df, set_df: 49;
    get_mf, set_mf: 50;
    get_fragment_offset, set_fragment_offset: 63, 51;
    get_time_to_live, set_time_to_live: 71, 64;
    get_protocol, set_protocol: 79, 72;
    get_header_checksum, set_header_checksum: 95, 80;
    u32, into Ipv4Address, get_source_address, set_source_address: 127, 96;
    u32, into Ipv4Address, get_destination_address, set_destination_address: 159, 128;
}
