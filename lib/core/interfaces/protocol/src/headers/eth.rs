use core::fmt;
use core::convert::TryInto;

pub struct HwAddress {
    addr: [u8; 6],
}

impl From<[u8; 6]> for HwAddress {
    fn from(octets: [u8; 6]) -> HwAddress {
        let mut hw: HwAddress = HwAddress { addr: [0u8; 6] };
        for (i, octets) in octets.iter().enumerate() {
            hw.addr[i] = *octets;
        }
        hw
    }
}

impl fmt::Debug for HwAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..=5 {
            if i == 5 {
                write!(f, "{:02x}", self.addr[i]).unwrap();
            } else {
                write!(f, "{:02x}:", self.addr[i]).unwrap();
            }
        }
        write!(f, "")
    }
}

impl From<u64> for HwAddress {
    fn from(mac: u64) -> HwAddress {
        let mac_slice: &[u8] = &mac.to_be_bytes()[2..=7];
        let mac_addr: [u8; 6] = mac_slice.try_into().expect("slice len incorrect");
        HwAddress::from(mac_addr)
    }
}

pub struct EtherType {
    typ: [u8; 2],
}

impl From<[u8; 2]> for EtherType {
    fn from(octets: [u8; 2]) -> EtherType {
        let mut t: EtherType = EtherType { typ: [0u8; 2] };
        for (i, octets) in octets.iter().enumerate() {
            t.typ[i] = *octets;
        }
        t
    }
}

impl fmt::Debug for EtherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}{:02x}", self.typ[0], self.typ[1])
    }
}

impl From<u64> for EtherType {
    fn from(etype: u64) -> EtherType {
        let ty_slice: &[u8] = &etype.to_be_bytes()[6..=7];
        let eth_typ: [u8; 2] = ty_slice.try_into().expect("slice len incorrect");
        EtherType::from(eth_typ)
    }
}

bitfield! {
    pub struct EthernetHeader(MSB0 [u8]);
    impl Debug;
    u64;
    // 6 bytes
    into HwAddress, get_dst_mac, set_dst_mac: 47, 0;
    // 6 bytes
    into HwAddress, get_src_mac, set_src_mac: 95, 48;
    // 2 bytes
    into EtherType, get_ether_type, set_ether_type: 111, 96;
}
