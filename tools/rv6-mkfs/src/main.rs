mod params;
mod fs;
mod utils;
mod handlers;

extern crate lazy_static;
use crate::fs::{SuperBlock, DINode, DirEntry, SectorHandler, NodeHandler};
// use crate::params::*;
use serde::{Deserialize, Serialize};
// use memcpy::{memcpy ,memmove, memset, memcmp};
use spin::Once;

use std::{
    vec::Vec,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write, Seek, SeekFrom},
    mem::{size_of},
};
use crate::handlers::{SectorHandler, NodeHandler};
use nix::dir::Dir;


fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

    let mut zeroes: [u8; params::BSIZE] = [0; params::BSIZE];

    let mut sector_handler = SectorHandler::new(&argv[1]);
    let mut node_handler = NodeHandler::new(&mut sector_handler);

    if argv.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }

    let nmeta: usize = 2 + nlog + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
            nmeta, params::LOGSIZE, params::NINODEBLOCKS, params::NBITMAP, nblocks, params::FSSIZE);
    let freeblock: usize = nmeta;

    for i in 0..params::FSSIZE {
       sector_handler.write_sector(1, &mut zeroes);
    }

    utils::fill(&mut buf, &node_handler.superblock_bytes(), 0);
    wsect(1, buf);

    let rootino: u32 = ialloc(1);
    let mut dir_entry = DirEntry::new(rootino as u16, ".");
    node_handler.append_inode(rootino, &mut dir_entry, size_of::<DirEntry>() as i32);

    for i in 2..argv.len() {
        if argv[i][0] == "_" {
            argv[i].next();
        }

        let inum = node_handler.alloc_inode(1);
        let de = DirEntry::new(inum as u16, &argv[i]);
        sector_handler.append_inode(inum, de, size_of::<de>());

        while let Ok(bytes) = utils::read_up_to(fd, &mut *buf) > 0 {
            let mut temp_de: DirEntry = bincode::deserialize(&*buf).unwrap();
            node_handler.append_inode(inum, &mut temp_de, bytes);
        }
    }

    let mut din = DINode::new();
    node_handler.read_inode(rootino, &mut din);
    let mut off = din.size;
    off = ((off / params::BSIZE) + 1) * params::BSIZE;
    node_handler.write_inode(rootino, &mut din);
    node_handler.alloc_block(freeblock as i32);
}
