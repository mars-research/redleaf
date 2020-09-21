bitfield! {
    pub struct UdpHeader(MSB0 [u8]);
    impl Debug;
    u32;
    // 2 bytes
    get_src_port, set_src_port: 15, 0;
    // 2 bytes
    get_dst_port, set_dst_port: 31, 16;
    // 2 bytes
    get_length, set_length: 47, 32;
    // 2 bytes
    get_chksum, set_chksum: 63, 48;
}
