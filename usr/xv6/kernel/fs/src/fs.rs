use byteorder::{ByteOrder, LittleEndian};
use libsyscalls::sysbdev;
use spin::Once;
use syscalls::BDevPtr;

use crate::params;
use crate::log::{Log, LOG};
use crate::icache::ICache;
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

// pub struct FileSystem {
//     pub superblock: SuperBlock,
//     pub bcache: BufferCache,
//     pub log: Log,
//     pub icache: ICache,
// }

// impl FileSystem {
//     // We only support a single device for now
//     pub fn new(dev: BDevPtr) -> Self {
//         let superblock = read_superblock(dev);
//         let log = Log::new(1234, &superblock);
//         Self {
//             superblock,
//             bcache: BufferCache::new(),
//             log,
//             icache: ICache::new(),
//         }
//     }
// }

// TODO: load super block from disk
fn read_superblock(dev: u32) -> SuperBlock {
    let mut buffer = BCACHE.read(dev, 1);
    let superblock = SuperBlock::from_bytes(&buffer.lock().data);
    BCACHE.release(&mut buffer);
    console::println!("Superblock read from disk: {:?}", superblock);
    superblock
}

// TODO: better name and place
pub fn block_num_for_node(inum: u16, super_block: &SuperBlock) -> u32 {
    return inum as u32 / params::IPB as u32 + super_block.inodestart;
}

pub fn fsinit(dev: u32) {	
    SUPER_BLOCK.call_once(|| read_superblock(dev));	
    LOG.call_once(|| {	
        Log::new(dev, SUPER_BLOCK.r#try().unwrap())	
    });	
} 
