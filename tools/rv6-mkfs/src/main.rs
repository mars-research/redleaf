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
    /* TODO: make NodeHandler accept a filename and init the Sectorhandler direcrtly from that. Then
        refactor main() to use nodeHandler.sector_handler
     */
    let mut argv: Vec<String> = std::env::args().collect();
    let mut buf: [u8; params::BSIZE] = [0; params::BSIZE];

    let mut zeroes: [u8; params::BSIZE] = [0; params::BSIZE];

    // let mut sector_handler = SectorHandler::new(&argv[1]);
    let mut node_handler = NodeHandler::new(&argv[1]);

    if argv.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }

    let nmeta: usize = 2 + params::LOGSIZE + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
            nmeta, params::LOGSIZE, params::NINODEBLOCKS, params::NBITMAP, nblocks, params::FSSIZE);
     node_handler.freeblock = nmeta as u32;

    for i in 0..params::FSSIZE {
        node_handler.write_file(i as u32, &mut zeroes);
       // sector_handler.write_sector(1, &mut zeroes);
    }

    utils::fill(&mut buf, &node_handler.superblock_bytes(), 0);
    node_handler.write_file(1, &mut buf);
    // sector_handler.write_sector(1, &mut buf);

    let rootino: u32 = node_handler.alloc_inode(1);
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



        let mut done = false;
        while !done {
           let read_res = utils::read_up_to(&mut fd, &mut buf);
           let bytes = match read_res   {
                Ok(b) => b,
                Err(_) => panic!(),
            };


            if bytes > 0 {
                // println!("\tbytes: {:?}", bytes);

                let mut temp_de: DirEntry = bincode::deserialize(&buf).unwrap();
                node_handler.append_inode(inum, &mut buf, bytes as i32);
            }
            else {
                done = true;
            }
        }

    }

    let mut din = DINode::new();
    node_handler.read_inode(rootino, &mut din);
    let mut off = din.size;
    off = ((off / params::BSIZE as u32) + 1) * params::BSIZE as u32;
    node_handler.write_inode(rootino, &mut din);
    node_handler.alloc_block(node_handler.freeblock as i32);
}
