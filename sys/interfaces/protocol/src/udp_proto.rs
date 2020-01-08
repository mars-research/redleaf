use hw_addr::EthernetHeader;
use ipv4::IpV4Header;
use udp::UdpHeader;

#[repr(u16)]
enum Protocol {
    Ipv4 = 0x0800,
}

