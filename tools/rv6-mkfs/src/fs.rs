use crate::params;
use serde::{Deserialize, Serialize};
use std::{
    mem::{size_of},
    fs::{File, OpenOptions},
    io::{Write, Read, Seek, SeekFrom},
};

#[derive(Debug)]
pub struct SuperBlock {
    pub size: u32,
    // Size of file system image (blocks)
    pub nblocks: u32,
    // Number of data blocks
    pub ninodes: u32,
    // Number of inodes.
    pub nlog: u32,
    // Number of log blocks
    pub logstart: u32,
    // Block number of first log block
    pub inodestart: u32,
    // Block number of first inode block
    pub bmapstart: u32, // Block number of first free map block
}

impl SuperBlock {
    pub fn new() -> SuperBlock {
        SuperBlock {
            size: 0u32,
            nblocks: 0u32,
            ninodes: 0u32,
            nlog: 0u32,
            logstart: 0u32,
            inodestart: 0u32,
            bmapstart: 0u32
        }
    }
    pub fn init() -> SuperBlock {
        let offset = 2;
        let nmeta: usize = 2 + nlog + params::NINODEBLOCKS + params::NBITMAP;
        let nblocks: usize = params::FSSIZE - nmeta;

        SuperBlock {
            size: params::FSSIZE as u32,
            nblocks: nblocks as u32,
            ninodes: params::NINODES as u32,
            nlog: params::LOGSIZE as u32,
            logstart: offset,
            inodestart: offset + params::LOGSIZE,
            bmapstart: offset + params::LOGSIZE + params::NINODEBLOCKS,
        }
    }
}
#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    inum: u16,
    name: [u8; params::DIRSIZE],
}

#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct INodeData {
    // File type
    pub file_type: i16,
    // Major device number (T_DEVICE only)
    pub major: i16,
    // Minor device number (T_DEVICE only)
    pub minor: i16,
    // Number of links to inode in file system
    pub nlink: i16,
    // Size of file (bytes)
    pub size: u32,
    // Data block addresses
    pub addresses: [u32; params::NDIRECT + 1],
}

impl INodeData {
    pub fn new() -> Self {
        Self {
            file_type: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addresses: [0; params::NDIRECT + 1],
        }
    }
}

pub type DINode = INodeData;

// Block containing inode i
pub fn iblock(i: u32, sb: &SuperBlock) {
    i / params::IPB as u32 + sb.inodestart;
}

#[derivce(Debug)]
pub struct NodeHandler {
    super_block: SuperBlock,
    sector_handler: &'a SectorHandler,
    dinode_size: usize,
    freeblock: u32,
}

impl INodeIO {
    pub fn new(s_handler: &'a SectorHandler) -> Self {
        DINodeHandler {
            sector_handler: s_handler,
            dinode_size: size_of::<DINode>(),
            freeblock: 0,
        }
    }

    pub fn iblock(&self, i: u32) {
        i / params::IPB as u32 + self.super_block.inodestart;
    }

    pub fn write_inode(&self, inum: u32, ip: &mut DINode) {
        let mut buffer = [0u8; params::BSIZE];
        self.block.read_sector(iblock(inum), &mut buffer);
        dinode.addresses[fbn] = self.freeblock;
        self.freeblock += 1;
    
        let offset = (inum as usize % params::IPB) * self.dinode_size;
        let slice = &mut buffer[offset..offset + self.dinode_size];
        ip = bincode::deserialize(&slice).unwrap();
        self.block.write_sector(bn, &mut buffer);
    }

    fn read_inode(&self, inum: u32, ip: &mut DINode) {
        let mut buf = &mut [0u8; params::BSIZE];
        self.sector_handler.read_sector(iblock(inum), buf);
        const DINODE_SIZE: usize = size_of::<DINode>();

        let dinode_offset = (inum as usize % params::IPB) * self.dinode_size;
        let dinode_slice = buf[dinode_offset..dinode_offset + self.dinode_size];
        ip = bincode::deserialize(&dinode_slice).unwrap();
    }
}

pub struct SectorHandler {
    file: File,
}

impl SectorHandler {
    pub fn new(filename: &String) -> Self {
        BlockIO {
            // turn into a match
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(filename).unwrap(),
        }
    }

    pub fn read_sector(&mut self, sec: u32, buf: &mut [u8]) {
        let mut f = File::open("foo.txt");
        f.seek(SeekFrom::Start(42));

        let block: u64 = sec as u64 * params::BSIZE as u64;
        if self.file.seek(SeekFrom::Start(block)).unwrap() != block {
            panic!("seek");
        }
    
        let bytes_read = self.file.read(buf).unwrap();
    
        if bytes_read != params::BSIZE {
            eprint!("error: read {} bytes. usually caused by not having enough space. 
                    increase FSZIE in params.rs to fix this. \n", bytes_read);
            panic!("read");
        }
    }

    pub fn write_sector(&mut self, sec :u32, buf: &mut [u8]) {
        // assert!(buf.len() == params::BSIZE);
        assert_eq!(buf.len(), params::BSIZE);

        let location: u64 = (sec * params::BSIZE) as u64;
        if self.file.seek(SeekFrom::Start(location)).unwrap() != location {
            panic!("seek");
        }
    
        if self.file.write(buf) != params::BSIZE {
            panic!("write");
        }
    }
    
}