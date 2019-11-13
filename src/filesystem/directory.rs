use crate::filesystem::params;

// Correspond to dirent in xv6
#[repr(C)]
pub struct DirectoryEntry {
    pub inum: u16,
    pub name: [u8; params::DIRSIZ],
}

