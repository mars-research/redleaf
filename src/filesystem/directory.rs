use core::convert::TryFrom;
use crate::common::bytearray; 
use crate::filesystem::params;

// Correspond to dirent in xv6
#[repr(C)]
pub struct DirectoryEntryDisk {
    pub inum: u16,
    pub name: [u8; params::DIRSIZ],
}

impl DirectoryEntryDisk {
    pub fn from_byte_array(arr: &[u8]) -> Self {
        Self {
            inum: bytearray::to_u16(arr),
            name: <[u8; params::DIRSIZ]>::try_from(&arr[2..2+params::DIRSIZ]).unwrap(),
        }
    }
}

// DirectoryEntryDisk but without copying the `name`
pub struct DirectoryEntry<'a> {
    pub inum: u16,
    pub name: &'a [u8],
}

impl<'a> DirectoryEntry<'a> {
    pub fn from_byte_array(arr: &'a [u8]) -> Self {
        Self {
            inum: bytearray::to_u16(&arr[..2]),
            name: &arr[2..2+params::DIRSIZ],
        }
    }
}

