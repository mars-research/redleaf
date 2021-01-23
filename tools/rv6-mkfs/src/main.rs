mod params;
mod fs;
mod block;
mod memcpy;
mod icache;

use crate::fs::{SuperBlock};
use crate::params::*;

use memcpy::{memcpy ,memmove, memset, memcmp};

use std::{
    vec::Vec,
    fs::File,
    io::{BufReader, BufWriter, Write,SeekFrom},
    os::unix::io::{FromRawFd, IntoRawFd},
};


const BSIZE : usize = 4096;
pub const FSSIZE: usize = 1000; // size of file system in blocks
mut freeinode: u32 = 1;
mut file: File;

fn wsect<T>(sec :u32, &mut T) {
    if libc::lseek(fsfd, sec * BSIZE, 0) != sec * BSIZE {

        std::process::exit(1);
    }
    if libc::write(fdfd, buf, BSIZE) != BISZE {
        std::process::exit(1);
    }
}

fn winode(inum: u32, ip: DINode) {
    
}

fn ialloc(type: INodeFileType) -> u32 {
    let inum: u32 = freeinode;
    freeinode += 1;

    let dinode = DINode::new();
    dinode.file_type = type;
    dinode.nlink = 1 as i16;
    dinode.size = 0 as u32;

    // winode(inum, dinode);
    inum;
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let buf: Vec<u8> = vec![0; BSIZE];

    if argv.len() < 2 {
        print!("Usage: mkfs fs.img files...\n");
    }

    // fsfd = nix::fcntl::open(argv[1], 
    //     O_CREAT | O_RDWR | O_TRUNC,
    //     0666);

    use std::fs::OpenOptions;

    file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(argv[1]);
    
    
    
    let nmeta: usize = 2 + nlog + NINODEBLOCKS + nbitmap;
    let nblocks: usize = FSSIZE - nmeta;

    // assert!(BSIZE % )
    let superblock = SuperBlock{
        size: FSSIZE,
        nblocks: nblocks,
        ninodes: NINODES,
        nlog: LOGSIZE,
        logstart: 2,
        inodestart: 2 + LOGSIZE,
        bmapstart: 2+LOGSIZE + NINODEBLOCKS,
    };

    print!("nmeta {} (boot, super, log blocks {} inode blocks {}, bitmapblocks {}) blocks {} total {}\n",
            nmeta, LOGSIZE, NINODEBLOCKS, nbitmap, nblocks, FSSIZE);
    let freeblock: usize = nmeta;

    for i in 0..FSSIZE {
        nix::unistd::wsect(1, buf);
    }

    // memset(buf, 0, std::mem::size_of(buf));
    memset(buf.as_mut_ptr(),
        0,
        std::mem::size_of(buf)
    );
    let bytes = unsafe { utils::to_bytes(&superblock) };
    std::ptr::copy(&superblock, &mut buf, std::mem::size_of(superblock));
    wsect(1, buf);

    let rootino: u32 = nix::unistd::ialloc();

}


// TODO: Replace serialize_into with...
// unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
//     ::std::slice::from_raw_parts(
//         (p as *const T) as *const u8,
//         ::std::mem::size_of::<T>(),
//     )
// }

// fn main() {
//     struct MyStruct {
//         id: u8,
//         data: [u8; 1024],
//     }
//     let my_struct = MyStruct { id: 0, data: [1; 1024] };
//     let bytes: &[u8] = unsafe { any_as_u8_slice(&my_struct) };
//     // tcp_stream.write(bytes);
//     println!("{:?}", bytes);
// }
