use byteorder::{ByteOrder, LittleEndian};
use core::convert::TryFrom;
use crate::params;

// Correspond to dirent in xv6
#[repr(C)]
pub struct DirectoryEntryDisk {
    pub inum: u16,
    pub name: [u8; params::DIRSIZ],
}

impl DirectoryEntryDisk {
    pub fn from_byte_array(arr: &[u8]) -> Self {
        Self {
            inum: LittleEndian::read_u16(arr),
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
            inum: LittleEndian::read_u16(&arr[..2]),
            name: &arr[2..2+params::DIRSIZ],
        }
    }

    pub fn as_bytes(&self) -> [u8; core::mem::size_of::<DirectoryEntry>()] {
        let mut arr = [0u8; core::mem::size_of::<DirectoryEntry>()];
        LittleEndian::write_u16(&mut arr[0..2], self.inum);
        arr[2..].copy_from_slice(self.name);
        arr
    }
}
