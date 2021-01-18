
use crate::fs::{SuperBlock, SUPER_BLOCK};
use crate::params::*;

const BSIZE : u32 = 4096;
pub const FSSIZE: usize = 1000; // size of file system in blocks

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }
    
    
    let nmeta: u32 = 2 + nlog + ninodeblocsk + nbitmap;
    let nblocks: u32 = FSSIZE - nmeta;

    // assert!(BSIZE % )
    let superblock = SuperBlock{
        size: FSSIZE,
        nblocks: nblocks,
        ninodes: NINODES,
        nlog: LOGSIZE,
        logstart: 2,
        inodestart: 2 + LOGSIZE,
        bmapstart: 2+LOGSIZE + NINODEBLOCKSs,
    };



}