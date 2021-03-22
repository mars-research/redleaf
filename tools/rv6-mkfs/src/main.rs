mod params;
mod fs;
mod utils;
mod inode;
mod layer;

use crate::fs::{FSHandler, SuperBlock,  DirEntry};
use crate::inode::DINode;
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
    let mut fshandler = FSHandler::new(&argv[1]);

    if argv.len() < 2 {
        panic!("Usage: mkfs fs.img files...\n");
    }

    let nmeta: usize = 2 + params::LOGSIZE + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
            nmeta, params::LOGSIZE, params::NINODEBLOCKS, params::NBITMAP, nblocks, params::FSSIZE);
     fshandler.freeblock = nmeta as u32;

    for i in 0..params::FSSIZE {
        fshandler.write_file(i as u32, &mut zeroes);
    }

    utils::fill(&mut buf, &fshandler.superblock_bytes(), 0);
    fshandler.write_file(1, &mut buf);

    let rootino: u32 = fshandler.alloc_inode(params::ROOTINO as i16);
    let mut dir_entry = DirEntry::new(rootino as u16, ".");
    fshandler.append_inode(rootino, &mut dir_entry.bytes(), size_of::<DirEntry>() as i32);

    let mut dir_entry = DirEntry::new(rootino as u16, "..");
    fshandler.append_inode(rootino, dir_entry.bytes(), size_of::<DirEntry>() as i32);

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

        let inum = fshandler.alloc_inode(2);
        let mut de = DirEntry::new(inum as u16, &arg);
        fshandler.append_inode(rootino, &mut de.bytes(), size_of::<DirEntry>() as i32);

        loop {
           let bytes= utils::read_up_to(&mut fd, &mut buf).unwrap();
            if bytes > 0 {
                fshandler.append_inode(inum, &mut buf, bytes as i32);
            }
            else {
                break;
            }
        }

    }

    let mut din = DINode::new();
    fshandler.read_inode(rootino, &mut din);
    
    let mut off = din.size;
    off = ((off / params::BSIZE as u32) + 1) * params::BSIZE as u32;
    din.size = off;

    fshandler.write_inode(rootino, &mut din);
    fshandler.alloc_disk_block(fshandler.freeblock as i32);
}
