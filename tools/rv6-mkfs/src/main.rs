mod params;
mod fs;
mod utils;
mod handlers;

use crate::fs::{SuperBlock, DINode, DirEntry};
use crate::handlers::{NodeHandler, SectorHandler};
use serde::{Deserialize, Serialize};

use std::{
    vec::Vec,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write, Seek, SeekFrom},
    mem::{size_of},
};
use std::path::Path;

fn main() {
    let mut argv: Vec<String> = std::env::args().collect();
    let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

    let mut zeroes: [u8; params::BSIZE] = [0; params::BSIZE];
    let mut node_handler = NodeHandler::new(&argv[1]);

    if argv.len() < 2 {
        panic!("Usage: mkfs fs.img files...\n");
    }

    let nmeta: usize = 2 + params::LOGSIZE + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
            nmeta, params::LOGSIZE, params::NINODEBLOCKS, params::NBITMAP, nblocks, params::FSSIZE);
     node_handler.freeblock = nmeta as u32;

    for i in 0..params::FSSIZE {
        node_handler.write_file(i as u32, &mut zeroes);
    }

    utils::fill(&mut buf, &node_handler.superblock_bytes(), 0);
    node_handler.write_file(1, &mut buf);

    let rootino: u32 = node_handler.alloc_inode(params::ROOTINO as i16);
    let mut dir_entry = DirEntry::new(rootino as u16, ".");
    node_handler.append_inode(rootino, &mut dir_entry.bytes(), size_of::<DirEntry>() as i32);

    let mut dir_entry = DirEntry::new(rootino as u16, "..");
    node_handler.append_inode(rootino, dir_entry.bytes(), size_of::<DirEntry>() as i32);

    for arg in argv.iter_mut().skip(2) {
        println!("adding {:?}", arg);
        assert!(!arg.contains("/"));
        let p = Path::new(&arg);
        let mut fd =  OpenOptions::new()
            .read(true)
            .open(&p).unwrap();

        if arg.chars().nth(0).unwrap() == '_' {
            arg.chars().next();
        }

        let inum = node_handler.alloc_inode(2);
        let mut de = DirEntry::new(inum as u16, &arg);
        node_handler.append_inode(rootino, &mut de.bytes(), size_of::<DirEntry>() as i32);

        loop {
           let bytes= utils::read_up_to(&mut fd, &mut buf).unwrap();
            if bytes > 0 {
                node_handler.append_inode(inum, &mut buf, bytes as i32);
            }
            else {
                break;
            }
        }

    }

    let mut din = DINode::new();
    node_handler.read_inode(rootino, &mut din);
    let mut off = din.size;
    off = ((off / params::BSIZE as u32) + 1) * params::BSIZE as u32;
    din.size = off;
    node_handler.write_inode(rootino, &mut din);
    node_handler.alloc_block(node_handler.freeblock as i32);
}
