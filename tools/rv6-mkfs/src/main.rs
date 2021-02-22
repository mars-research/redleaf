mod params;
mod fs;

extern crate lazy_static;
use crate::fs::{SuperBlock, DINode, DirEntry};
// use crate::params::*;
use serde::{Deserialize, Serialize};
// use memcpy::{memcpy ,memmove, memset, memcmp};
use spin::Once;

use std::{
    vec::Vec,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write, Seek, SeekFrom},
    os::unix::io::{FromRawFd, IntoRawFd},
    mem::{size_of},
};


fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let buf: Vec<u8> = vec![0; params::BSIZE];

    if argv.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }



    let nmeta: usize = 2 + nlog + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
            nmeta, params::LOGSIZE, params::NINODEBLOCKS, params::NBITMAP, nblocks, params::FSSIZE);
    let freeblock: usize = nmeta;

    for i in 0..params::FSSIZE {
       wsect(1, buf);
    }

    utils::zero(&mut buf);


    let bytes = unsafe { utils::to_bytes(&sb) };
    std::ptr::copy(&sb, &mut buf, std::mem::size_of::<SuperBlock>());
    wsect(1, buf);

    let rootino: u32 = ialloc(T_DIR);


}
