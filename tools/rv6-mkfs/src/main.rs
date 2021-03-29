#[macro_use]
extern crate num_derive;

use crate::fs::{DirEntry, FSHandler};
use crate::inode::{DINode, INodeFileType};
use crate::superblock::SuperBlock;
use crate::params::BSIZE;

use std::path::Path;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    mem::size_of,
    vec::Vec,
};

mod fs;
mod inode;
mod layer;
mod params;
mod superblock;
mod utils;

fn main() {
    let mut argv: Vec<String> = std::env::args().collect();
    if argv.len() < 2 {
        panic!("Usage: mkfs fs.img files...\n");
    }

    let mut buf: [u8; BSIZE] = [0; BSIZE];
    let mut zeroes: [u8; BSIZE] = [0; BSIZE];
    let mut fs = FSHandler::new(&argv[1]);

    let nmeta: usize = 2 + params::LOGSIZE + params::NINODEBLOCKS + params::NBITMAP;
    let nblocks: usize = params::FSSIZE - nmeta;

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
           nmeta, params::LOGSIZE, params::NINODEBLOCKS, params::NBITMAP, nblocks, params::FSSIZE);
    fs.set_freeblock(nmeta as u32);

    for i in 0..params::FSSIZE {
        fs.write(i as u32, &mut zeroes);
    }

    // write superblock
    utils::fill(&mut buf, &fs.superblock_bytes(), 0);
    fs.write(1, &mut buf);

    // append . and ..
    let rootino = append_root(&mut fs);

    // append each additional file
    for arg in argv.iter_mut().skip(2) {
        println!("adding {:?}", arg);
        assert!(!arg.contains("/"));
        append_file(rootino, arg, &mut fs);
    }

    let mut din = DINode::new();
    fs.read_inode(rootino, &mut din);

    let mut off = din.size;
    off = ((off / BSIZE as u32) + 1) * BSIZE as u32;
    din.size = off;

    fs.write_inode(rootino, &mut din);
    fs.alloc_disk_block(fs.get_freeblock() as i32);
}

pub fn append_root(fs: &mut FSHandler) -> u32 {
    /// Appends .. and . directories to the SAME inode. At the root of the filesystem,
    /// .. and . BOTH refer to the root.
    ///
    /// Inode numbers start at  since 0 was used as a return value  when an inode
    /// could not be found
    let rootino: u32 = fs.alloc_inode(INodeFileType::Directory);
    assert_eq!(rootino, params::ROOTINO as u32);
    let mut dir_entry = DirEntry::new(rootino as u16, ".");

    fs.append_data_to_inode(rootino, &mut dir_entry.bytes());

    let mut dir_entry = DirEntry::new(rootino as u16, "..");
    fs.append_data_to_inode(rootino, dir_entry.bytes());

    rootino
}

pub fn append_file(root_inum: u32, file_path: &String, fs: &mut FSHandler) {
    /// Appends additional, arbitrary files to the  root inode of file system
    /// by creating a new directory entry for each file, adding that directory entry to the
    /// filesystem root, and then adding the file's contents to that directory entry
    ///
    /// # Arguments
    /// * `root_inum` - The inode number of the inode at the root of the filesystem
    /// * `file_path` - The filepath of the file whose data we want to append
    /// * `fs` - Filesystem handler object

    let mut buf: [u8; BSIZE] = [0; BSIZE];

    let p = Path::new(&file_path);
    let mut file = OpenOptions::new().read(true).open(&p).unwrap();

    if file_path.chars().nth(0).unwrap() == '_' {
        file_path.chars().next();
    }

    // Allocate a new inode for a directry entry
    let inum = fs.alloc_inode(INodeFileType::File);
    let mut de = DirEntry::new(inum as u16, &file_path);

    // append directory entry to the root inode (1) of the filesystem
    fs.append_data_to_inode(root_inum, &mut de.bytes());

    // append the file's contents to the inum of the directory entry
    // that we just created
    loop {
        let bytes = utils::read_up_to(&mut file, &mut buf).unwrap();
        if bytes > 0 {
            fs.append_data_to_inode(inum, &mut buf[..bytes]);
        } else {
            break;
        }
    }
}
