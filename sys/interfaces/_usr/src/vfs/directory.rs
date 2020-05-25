use byteorder::{ByteOrder, LittleEndian};
use core::convert::TryFrom;

/// Max size of directory name
const DIRSIZ: usize = 14;

// Correspond to dirent in xv6
#[repr(C)]
#[derive(Debug)]
pub struct DirectoryEntry {
    pub inum: u16,
    pub name: [u8; DIRSIZ],
}

impl DirectoryEntry {
    pub fn from_bytes(arr: &[u8]) -> Self {
        Self {
            inum: LittleEndian::read_u16(arr),
            name: <[u8; DIRSIZ]>::try_from(&arr[2..2+DIRSIZ]).unwrap(),
        }
    }
}

// DirectoryEntry but without copying the `name`
#[derive(Debug)]
pub struct DirectoryEntryRef<'a> {
    pub inum: u16,
    pub name: &'a [u8],
}

impl<'a> DirectoryEntryRef<'a> {
    pub fn from_bytes(arr: &'a [u8]) -> Self {
        Self {
            inum: LittleEndian::read_u16(&arr[..2]),
            name: &arr[2..2+DIRSIZ],
        }
    }

    pub fn as_bytes(&self) -> [u8; core::mem::size_of::<DirectoryEntry>()] {
        let mut bytes = [0u8; core::mem::size_of::<DirectoryEntry>()];
        LittleEndian::write_u16(&mut bytes[0..2], self.inum);
        bytes[2..].copy_from_slice(self.name);
        bytes
    }
}

extern crate red_idl;

red_idl::declare_safe_copy!(DirectoryEntry);
red_idl::declare_safe_copy!(DirectoryEntryRef < 'a >);
