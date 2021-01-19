use std::fs::File;
use crate::fs::{SuperBlock, SUPER_BLOCK};
use crate::params::*;
use std::io::SeekFrom;

const BSIZE : usize = 4096;
pub const FSSIZE: usize = 1000; // size of file system in blocks

fn main() {
    let argv: Vec<String> = std::env::args().collect();

    if argv.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }
    
    // create make a new file if current one doesnt not exist
    // and truncate file if it does exist
    let mut f = File::create(argv[i]); 

    let f = match f {
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };

    
    let nmeta: usize = 2 + nlog + ninodeblocsk + nbitmap;
    let nblocks: usize = FSSIZE - nmeta;

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