use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

use fnv::FnvHasher;

const ETH_HEADER_LEN: usize = 14;
const UDP_HEADER_LEN: usize = 8;

// https://en.wikipedia.org/wiki/IPv4
const IPV4_PROTO_OFFSET: usize = 9;
const IPV4_SRCDST_OFFSET: usize = 12;
const IPV4_SRCDST_LEN: usize = 8;

pub fn swap_mac(frame: &mut [u8]) {
    for i in 0..6 {
        frame.swap(i, 6 + i);
    }
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