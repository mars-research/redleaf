use std::mem;

use crate::{layer, params};
use byteorder::{ByteOrder, LittleEndian};
use num_traits::FromPrimitive;

#[repr(u16)]
#[derive(PartialEq, Copy, Clone, Debug, FromPrimitive)]
pub enum INodeFileType {
    // This is not a file type; it indicates that the inode is not initialized
    Unitialized,
    // Correspond to T_DIR in xv6
    Directory,
    // Correspond to T_FILE in xv6
    File,
    // Correspond to
    Device,
}

#[repr(C)]
#[derive(Debug)]
pub struct INodeData {
    // File type
    pub file_type: INodeFileType,
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
            file_type: INodeFileType::Unitialized,
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
        self.file_type = FromPrimitive::from_u16(file_type).unwrap();

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
