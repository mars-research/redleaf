use crate::{params, utils};
use serde::{Deserialize, Serialize};
use std::{mem::{size_of}, fs::{File, OpenOptions}, io::{Write, Read, Seek, SeekFrom}, ops, slice, mem};

#[derive(Debug, Copy, Clone)]
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
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const SuperBlock as *const u8, mem::size_of::<SuperBlock>())
                as &[u8]
        }
    }
}

impl ops::Deref for SuperBlock {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const SuperBlock as *const u8, mem::size_of::<SuperBlock>())
                as &[u8]
        }
    }
}


#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    inum: u16,
    pub name: [u8; params::DIRSIZE],
}

impl DirEntry {
    pub fn default() -> Self {
        DirEntry {
            inum: 0,
            name: [0; params::DIRSIZE],
        }
    }

    pub fn new(n: u16, string: &str) -> Self {
        let mut dir = DirEntry {
            inum: n,
            name: [0; params::DIRSIZE],
        };

        let str_bytes = string.as_bytes();
        let mut i = 0;
        for byte in str_bytes {
            dir.name[i] = *byte;
            i += 1;
        }

        dir
    }

    pub fn change_name(&self, string: &str) {
        let str_bytes = string.as_bytes();
        let mut i = 0;
        for byte in str_bytes {
            self.name[i] = *byte;
            i += 1;
        }

    }

    pub fn bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const DirEntry as *const u8, mem::size_of::<DirEntry>())
                as &[u8]
        }
    }
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
pub type DINode = INodeData;

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

// impl ops::Deref for INodeData {
//     type Target = [u8];
//     fn deref(&self) -> &[u8] {
//         unsafe {
//             slice::from_raw_parts(self as *const INodeData as *const u8, mem::size_of::<INodeData>())
//                 as &[u8]
//         }
//     }
// }

