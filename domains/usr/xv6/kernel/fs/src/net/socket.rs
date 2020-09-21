use usr_interface::xv6::File;

pub struct Socket {
    address: [u8; 4],
    port: u16,
}
