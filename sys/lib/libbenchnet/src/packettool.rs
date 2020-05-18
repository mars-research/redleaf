use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

use fnv::FnvHasher;

const ETH_HEADER_LEN: usize = 14;
const UDP_HEADER_LEN: usize = 8;

// https://en.wikipedia.org/wiki/IPv4
const IPV4_PROTO_OFFSET: usize = 9;
const IPV4_LENGTH_OFFSET: usize = 2;
const IPV4_CHECKSUM_OFFSET: usize = 10;
const IPV4_SRCDST_OFFSET: usize = 12;
const IPV4_SRCDST_LEN: usize = 8;

const UDP_LENGTH_OFFSET: usize = 4;
const UDP_CHECKSUM_OFFSET: usize = 6;

pub fn swap_mac(frame: &mut [u8]) {
    for i in 0..6 {
        frame.swap(i, 6 + i);
    }
}

pub fn fix_ip_length(frame: &mut [u8]) {
    let length = frame.len() - ETH_HEADER_LEN;
    frame[ETH_HEADER_LEN + IPV4_LENGTH_OFFSET] = (length >> 8) as u8;
    frame[ETH_HEADER_LEN + IPV4_LENGTH_OFFSET + 1] = length as u8;
}

pub fn fix_ip_checksum(frame: &mut [u8]) {
    // Length of IPv4 header
    let v4len = (frame[ETH_HEADER_LEN] & 0b1111) as usize * 4;

    let checksum = calc_ipv4_checksum(&frame[ETH_HEADER_LEN..(ETH_HEADER_LEN + v4len)]);

    // Calculated checksum is little-endian; checksum field is big-endian
    frame[ETH_HEADER_LEN + IPV4_CHECKSUM_OFFSET] = (checksum >> 8) as u8;
    frame[ETH_HEADER_LEN + IPV4_CHECKSUM_OFFSET + 1] = (checksum & 0xff) as u8;
}

pub fn fix_udp_length(frame: &mut [u8]) {
    // Length of IPv4 header
    let v4len = (frame[ETH_HEADER_LEN] & 0b1111) as usize * 4;

    let length = frame.len() - ETH_HEADER_LEN - v4len;

    frame[ETH_HEADER_LEN + v4len + UDP_LENGTH_OFFSET] = (length >> 8) as u8;
    frame[ETH_HEADER_LEN + v4len + UDP_LENGTH_OFFSET + 1] = length as u8;
}

pub fn fix_udp_checksum(frame: &mut [u8]) {
    // Length of IPv4 header
    let v4len = (frame[ETH_HEADER_LEN] & 0b1111) as usize * 4;

    frame[ETH_HEADER_LEN + v4len + UDP_CHECKSUM_OFFSET] = 0;
    frame[ETH_HEADER_LEN + v4len + UDP_CHECKSUM_OFFSET + 1] = 0;
}

pub fn get_flowhash(frame: &[u8]) -> Option<usize> {
    // Ugly but fast (supposedly)
    let h1f: BuildHasherDefault<FnvHasher> = Default::default();
    let mut h1 = h1f.build_hasher();

    if frame[ETH_HEADER_LEN] >> 4 != 4 {
        // This shitty implementation can only handle IPv4 :(
        return None
    }

    // Length of IPv4 header
    let v4len = (frame[ETH_HEADER_LEN] & 0b1111) as usize * 4;

    // Hash source/destination IP addresses
    frame[(ETH_HEADER_LEN + IPV4_SRCDST_OFFSET)..(ETH_HEADER_LEN + IPV4_SRCDST_OFFSET + IPV4_SRCDST_LEN)].hash(&mut h1);

    // Hash IP protocol number
    let proto = frame[ETH_HEADER_LEN + IPV4_PROTO_OFFSET];
    if proto != 6 && proto != 17 {
        // This shitty implementation can only handle TCP and UDP
        return None;
    }
    proto.hash(&mut h1);

    // Hash source/destination port
    frame[(ETH_HEADER_LEN + v4len)..(ETH_HEADER_LEN + v4len + 4)].hash(&mut h1);

    Some(h1.finish() as usize)
}

pub fn get_mut_udp_payload(frame: &mut [u8]) -> Option<(usize, &mut [u8])> {
    if frame[ETH_HEADER_LEN] >> 4 != 4 {
        // This shitty implementation can only handle IPv4 :(
        return None
    }

    // Length of IPv4 header
    let v4len = (frame[ETH_HEADER_LEN] & 0b1111) as usize * 4;

    // Check IP protocol number
    let proto = frame[ETH_HEADER_LEN + IPV4_PROTO_OFFSET];
    if proto != 17 {
        // UDP only sorry
        return None;
    }

    Some((ETH_HEADER_LEN + v4len + UDP_HEADER_LEN, &mut frame[(ETH_HEADER_LEN + v4len + UDP_HEADER_LEN)..]))
}

pub fn swap_udp_ips(frame: &mut [u8]) {
    // Swaps both the IPs in the IPv4 header and the ports in the UDP header
    let v4len = (frame[ETH_HEADER_LEN] & 0b1111) as usize * 4;

    // UDP ports
    for i in 0..2 {
        frame.swap(ETH_HEADER_LEN + v4len + i, ETH_HEADER_LEN + v4len + 2 + i);
    }

    // IP addresses
    for i in 0..4 {
        frame.swap(ETH_HEADER_LEN + 12 + i, ETH_HEADER_LEN + 12 + 4 + i);
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

