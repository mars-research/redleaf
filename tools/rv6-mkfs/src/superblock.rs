use crate::params;
use std::{mem, slice};

// #[derive(Debug, Copy, Clone)]
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
            bmapstart: 0u32,
        }
    }
    pub fn init() -> SuperBlock {
        let offset = 2;
        let nmeta: usize = 2 + params::LOGSIZE + params::NINODEBLOCKS + params::NBITMAP;
        let nblocks: usize = params::FSSIZE - nmeta;

        SuperBlock {
            size: params::FSSIZE as u32,
            nblocks: nblocks as u32,
            ninodes: params::NINODES as u32,
            nlog: params::LOGSIZE as u32,
            logstart: offset,
            inodestart: offset + params::LOGSIZE as u32,
            bmapstart: offset + params::LOGSIZE as u32 + params::NINODEBLOCKS as u32,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const SuperBlock as *const u8,
                mem::size_of::<SuperBlock>(),
            ) as &[u8]
        }
    }
}
