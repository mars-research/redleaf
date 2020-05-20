use alloc::boxed::Box;
use byteorder::{ByteOrder, LittleEndian};
use spin::Once;

use usr_interface::bdev::BDev;

use crate::params;
use crate::log::{Log, LOG};
use crate::bcache::{BCACHE, BufferCache};

pub static SUPER_BLOCK: Once<SuperBlock> = Once::new();

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


// TODO: load super block from disk
fn read_superblock(dev: u32) -> SuperBlock {
    let mut buffer = BCACHE.r#try().unwrap().read(dev, 1);
    let superblock = SuperBlock::from_bytes(&***buffer.lock());
        console::println!("Superblock read from disk: {:?}", superblock);
    superblock
}

// TODO: better name and place
pub fn block_num_for_node(inum: u16, super_block: &SuperBlock) -> u32 {
    inum as u32 / params::IPB as u32 + super_block.inodestart
}

pub fn fsinit(dev_no: u32, dev: Box<dyn BDev>) {
    BCACHE.call_once(|| BufferCache::new(dev));
    SUPER_BLOCK.call_once(|| read_superblock(dev_no));	
    LOG.call_once(|| {	
        Log::new(dev_no, SUPER_BLOCK.r#try().unwrap())	
    });	
} 
