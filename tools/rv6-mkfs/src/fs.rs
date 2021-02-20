use crate::params;
use serde::{Deserialize, Serialize};
use std::{
    mem::{size_of},
    fs::{File},
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
    
    // pub fn from_bytes(bytes: &[u8]) -> Self {
    //     Self {
    //         size: LittleEndian::read_u32(&bytes[0..4]),
    //         nblocks: LittleEndian::read_u32(&bytes[4..8]),
    //         ninodes: LittleEndian::read_u32(&bytes[8..12]),
    //         nlog: LittleEndian::read_u32(&bytes[12..16]),
    //         logstart: LittleEndian::read_u32(&bytes[16..20]),
    //         inodestart: LittleEndian::read_u32(&bytes[20..24]),
    //         bmapstart: LittleEndian::read_u32(&bytes[24..28]),
    //     }
    // }
}

// impl Iter for SuperBlock {
//     impl<'a, T> IntoIterator for &'a mut Vec<T> {
//         // impl iterator that returns buffer to next dinode
//         type Item = DINode;

//         // TODO
//         fn next(&mut self) -> Option<Self::Item> {
//             let bguard = BCACHE
//                 .r#try()
//                 .unwrap()
//                 .read(device, block_num_for_node(inum, self));
//             let mut buffer = bguard.lock();

//             const DINODE_SIZE: usize = mem::size_of::<DINode>();
//             let offset = (inum as usize % params::IPB) * DINODE_SIZE;
//             // let slice = &mut buffer[offset..offset + DINODE_SIZE];
//             // let mut dinode = bincode::deserialize(&slice).unwrap();
        
//             // Some(dinode);
//         }
//     }
// }
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
// pub const DINODE_SIZE: usize = size_of::<DINode>();

// Block containing inode i
pub fn iblock(i: u32, sb: &SuperBlock) {
    i / params::IPB as u32 + sb.inodestart;
}

pub struct INodeIO {
    dinode: DINode,
    file: File,
}

impl INodeIO {
    pub fn write_inode(file: &mut File, inum: u32, ip: &DINode) {
        let mut buffer = [0u8; params::BSIZE];
    
        let bn = fs::iblock(inum, sb.get_mut());
        read_sector(file, bn, &mut buffer);
                    unsafe {
                        dinode.addresses[fbn] = freeblock;
                        freeblock += 1;
                    }
        const DINODE_SIZE: usize = size_of::<DINode>();
    
        let offset = (inum as usize % params::IPB) * DINODE_SIZE;
        let slice = &mut buffer[offset..offset + DINODE_SIZE];
        // let mut dinode = DINode::from_bytes(slice);
        let dinode = bincode::deserialize(&slice).unwrap();
        write_sector(file, bn, buffer);
    
        // Ok(off
}

pub struct BlockIO {

}