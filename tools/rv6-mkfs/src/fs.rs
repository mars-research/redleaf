use crate::{params, utils};
use serde::{Deserialize, Serialize};
use std::{mem::{size_of}, fs::{File, OpenOptions}, io::{Write, Read, Seek, SeekFrom}, ops, slice, mem};
use byteorder::{ByteOrder, LittleEndian};


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

    // pub fn change_name(&self, string: &str) {
    //     let str_bytes = string.as_bytes();
    //     let mut i = 0;
    //     for byte in str_bytes {
    //         self.name[i] = *byte;
    //         i += 1;
    //     }
    //
    // }

    pub fn bytes(&mut self) -> & mut [u8] {
        unsafe {
            slice::from_raw_parts_mut( self as *mut DirEntry as *mut u8, mem::size_of::<DirEntry>())
                as &mut [u8]
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

    pub fn new_from(rhs: &DINode) -> Self {
        Self {
            file_type: rhs.file_type,
            major: rhs.major,
            minor: rhs.minor,
            nlink: rhs.nlink,
            size: rhs.size,
            addresses: rhs.addresses.clone(),
        }
    }
    pub fn copy_from_bytes(&mut self, bytes: &[u8]) {
        let mut offset: usize = 0;
        let file_type = LittleEndian::read_u16(&bytes[offset..]);
        self.file_type = file_type as i16;
        offset += mem::size_of_val(&self.file_type);

        self.major = LittleEndian::read_i16(&bytes[offset..]);
        offset += mem::size_of_val(&self.major);

        self.minor = LittleEndian::read_i16(&bytes[offset..]);
        offset += mem::size_of_val(&self.minor);

        self.nlink = LittleEndian::read_i16(&bytes[offset..]);
        offset += mem::size_of_val(&self.nlink);

        self.size = LittleEndian::read_u32(&bytes[offset..]);
        offset += mem::size_of_val(&self.size);

        for a in &mut self.addresses {
            *a = LittleEndian::read_u32(&bytes[offset..]);
            offset += mem::size_of_val(a);
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut dinode = Self::new();
        dinode.copy_from_bytes(bytes);
        dinode
    }

    pub fn to_bytes(&self, bytes: &mut [u8]) {
        let mut offset: usize = 0;
        LittleEndian::write_u16(&mut bytes[offset..], self.file_type as u16);
        offset += mem::size_of_val(&self.file_type);

        LittleEndian::write_i16(&mut bytes[offset..], self.major);
        offset += mem::size_of_val(&self.major);

        LittleEndian::write_i16(&mut bytes[offset..], self.minor);
        offset += mem::size_of_val(&self.minor);

        LittleEndian::write_i16(&mut bytes[offset..], self.nlink);
        offset += mem::size_of_val(&self.nlink);

        LittleEndian::write_u32(&mut bytes[offset..], self.size);
        offset += mem::size_of_val(&self.size);

        for a in &self.addresses {
            LittleEndian::write_u32(&mut bytes[offset..], *a);
            offset += mem::size_of_val(a);
        }
    }

}

