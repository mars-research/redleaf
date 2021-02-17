mod params;
mod fs;

use crate::fs::{SuperBlock, DINode, DINODE_SIZE};
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


// static mut SUPERBLOCK: SuperBlock = SuperBlock::new();
static mut freeinode: u32 = 1; // TODO: Change this so it is not global
pub static sb: Once<SuperBlock> = Once::new();
static mut freeblock: u32 = 0; // TODO: Change this so it is not global
// static mut file: File; // TODO: Change this so it is not global
// static mut file: Option<File> = None;

fn write_sector(file: &mut File, sec :u32, buf: &mut [u8]) {
    assert!(buf.len() == params::BSIZE);

    if file.seek(SeekFrom::Start(sec * params::BSIZE)) != sec * params::BSIZE {
        panic!("seek");
    }

    if file.write(buf) != params::BSIZE {
        panic!("write");
    }
}

fn write_inode(file: &mut File, inum: u32, ip: &DINode) {
    let mut buffer = [0u8; params::BSIZE];

    let bn = fs::iblock(inum, sb);
    read_sector(file, bn, buffer);

    // const DINODE_SIZE: usize = mem::size_of::<DINode>();
    let offset = (inum as usize % params::IPB) * DINODE_SIZE;
    let slice = &mut buffer[offset..offset + DINODE_SIZE];
    // let mut dinode = DINode::from_bytes(slice);
    let dinode = bincode::deserialize(&slice).unwrap();
    write_sector(file, bn, buffer);

    // Ok(offset + DINODE_SIZE);
}

fn read_inode(inum: u32, ip: &mut DINode) {
    let mut buf = &mut [0u8; params::BSIZE];
    let bn = fs::iblock(inum, sb);

    read_sector(file, bn, buf);
    let dinode_offset = (inum as usize % params::IPB) * DINODE_SIZE;
    let dinode_slice = buf[dinode_offset..dinode_offset + DINODE_SIZE];
    ip = bincode::deserialize(&dinode_slice).unwrap();
}

fn read_sector(file: &mut File, sec: u32, buf: &mut [u8]) {
    if file.seek(SeekFrom::Start(sec * params::BSIZE)) != sec * params::BSIZE {
        panic!("seek");
    }

    let bytes_read = file.read_exact(buf);

    if bytes_read != params::BSIZE {
        eprint!("error: read {} bytes. usually caused by not having enough space. 
                increase FSZIE in params.rs to fix this. \n", bytes_read);
        panic!("read");
    }
}

fn ialloc(t: i16) -> u32 {
    let inum: u32 = freeinode;
    freeinode += 1;

    let dinode = DINode::new();
    dinode.file_type = t;
    dinode.nlink = 1 as i16;
    dinode.size = 0 as u32;
    winode(inum, dinode);

    inum;
}

fn balloc(file: &mut File, used: i32) {
    let mut buf = [0u8; params::BSIZE];

    for block_offset in 0..params::NBITMAP {
        if used <= 0 {
            return;
        }

        for elem in buf.iter_mut() { *elem = 0; }
        let nbits: i32 = if used > params::BPB as i32 { params::BPB as i32 } else { used };
        
        for bi in 0..nbits {
            let m = 1 << (bi % 8);
            let index : usize = bi / 8;
            buf[index] |= m; // mark block as used
        }
        write_sector(file, sb.bmapstart + block_offset, buf);
    }
}

fn append_inode(inum: u32, xp, n: i32) {
    //TODO: should xp be a buffer or a dirent?
    let mut dinode = DINode::new();
    read_inode(inum, dinode);
    let offset = dinode.size;

    while n > 0 {
        let fbn: usize = offset / params::BSIZE as u32;

        if fbn < params::NDIRECT as u32 {
            // Direct
            if dinode.addrs[fbn] == 0 {
                unsafe {
                    dinode.addrs[fbn] = freeblock;
                    freeblock += 1;
                }
            
            x = dinode.addrs[fbn];
        }
        }
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let buf: Vec<u8> = vec![0; params::BSIZE];

    if argv.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }

    // unsafe {
    //     file = OpenOptions::new()
    //         .read(true)
    //         .write(true)
    //         .create(true)
    //         .truncate(true)
    //         .open(argv[1]);
    // }
    
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(argv[1]);
    
    let nmeta: usize = 2 + nlog + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    sb.size = params::FSSIZE;
    sb.nblocks = nblocks;
    sb.ninodes = params::NINODES;
    sb.nlog = params::LOGSIZE;
    sb.logstart = 2;
    sb.inodestart = 2 + params::LOGSIZE;
    sb.bmapstart = 2 + params::LOGSIZE + params::NINODEBLOCKS;

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
