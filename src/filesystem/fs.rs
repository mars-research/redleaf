use alloc::sync::Arc;
use crate::filesystem::params;

const NINODES: usize = 200;

pub struct SuperBlock {
    pub size: u32,         // Size of file system image (blocks)
    pub nblocks: u32,      // Number of data blocks
    pub ninodes: u32,      // Number of inodes.
    pub nlog: u32,         // Number of log blocks
    pub logstart: u32,     // Block number of first log block
    pub inodestart: u32,   // Block number of first inode block
    pub bmapstart: u32,    // Block number of first free map block
}

// Hardcoded superblock
pub fn getSuperBlock() -> Arc<SuperBlock> {
    let nbitmap = params::FSSIZE / (params::BSIZE*8) + 1;
    let ninodeblocks = NINODES / params::IPB + 1;
    let nlog = params::LOGSIZE;

    // 1 fs block = 1 disk sector
    let nmeta = 2 + nlog + ninodeblocks + nbitmap;
    let nblocks = params::FSSIZE - nmeta;
    // TODO: ensure the encoding is intel's encoding
    Arc::new(SuperBlock {
        size: params::FSSIZE,
        nblocks,
        ninodes: NINODES,
        nlog, 
        logstart: 2,
        inodestart: 2 + nlog,
        bmapstart: 2 + nlog + ninodeblocks,
    })
} 
