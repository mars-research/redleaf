
bitflags! {
    pub struct FileMode: u32 {
        const READ = 0b001;
        const WRITE = 0b010;
        const CREATE = 0b100;
        const READWRITE = Self::READ.bits | Self::WRITE.bits;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FileStat {
    pub device: u32,
    pub inum: u16,
    pub file_type: INodeFileType,
    pub nlink: i16,
    pub size: u64,
}

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
