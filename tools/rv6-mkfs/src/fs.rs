use crate::params;

#[derive(Debug)]
pub struct SuperBlock {
    pub size: u32,
    // Size of file system image (blocks)
    pub nblocks: u32,
    // Number of data blocks
    pub ninodes: u32,
    // Number of inodes.
    pub nlog: u32,
    // Number of log blocks
    pub logstart: u32,
    // Block number of first log block
    pub inodestart: u32,
    // Block number of first inode block
    pub bmapstart: u32, // Block number of first free map block
}

impl SuperBlock {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            size: LittleEndian::read_u32(&bytes[0..4]),
            nblocks: LittleEndian::read_u32(&bytes[4..8]),
            ninodes: LittleEndian::read_u32(&bytes[8..12]),
            nlog: LittleEndian::read_u32(&bytes[12..16]),
            logstart: LittleEndian::read_u32(&bytes[16..20]),
            inodestart: LittleEndian::read_u32(&bytes[20..24]),
            bmapstart: LittleEndian::read_u32(&bytes[24..28]),
        }
    }
}

