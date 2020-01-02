use spin::Once;
use syscalls::BDevPtr;

use crate::params;
use crate::log::{Log, LOG};
use crate::icache::ICache;
use crate::bcache::BufferCache;

pub static SUPER_BLOCK: Once<SuperBlock> = Once::new();

pub struct SuperBlock {
    pub size: usize,
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

pub struct FileSystem {
    pub superblock: SuperBlock,
    pub bcache: BufferCache,
    pub log: Log,
    pub icache: ICache,
}

impl FileSystem {
    // We only support a single device for now
    pub fn new(dev: BDevPtr) -> Self {
        let superblock = read_superblock(dev);
        let log = Log::new(1234, &superblock);
        Self {
            superblock,
            bcache: BufferCache::new(),
            log,
            icache: ICache::new(),
        }
    }
}

// TODO: load super block from disk
fn read_superblock(dev: BDevPtr) -> SuperBlock {
    const NINODES: usize = 200;

    let nbitmap = params::FSSIZE / (params::BSIZE * 8) + 1;
    let ninodeblocks = NINODES / params::IPB + 1;
    let nlog = params::LOGSIZE;

    // 1 fs block = 1 disk sector
    let nmeta = 2 + nlog + ninodeblocks + nbitmap;
    let nblocks = params::FSSIZE - nmeta;
    // TODO: ensure the encoding is intel's encoding
    SuperBlock {
        size: params::FSSIZE as usize,
        nblocks: nblocks as u32,
        ninodes: NINODES as u32,
        nlog: nlog as u32,
        logstart: 2,
        inodestart: 2 + nlog as u32,
        bmapstart: (2 + nlog + ninodeblocks) as u32,
    }
}

// TODO: better name and place
pub fn block_num_for_node(inum: u32, super_block: &SuperBlock) -> u32 {
    return inum / params::IPB as u32 + super_block.inodestart;
}
