/// Virtual file system interface
/// Implemented by xv6 file system
use alloc::boxed::Box;
use alloc::vec::Vec;

pub use crate::vfs::file::{FileMode, FileStat, INodeFileType};
pub use crate::vfs::directory::{DirectoryEntry, DirectoryEntryRef};

pub mod file;
pub mod directory;

pub const NFILE: usize =       100;     // open files per system

// syscalls that are exposed to both the kernel and the users
pub trait UsrVFS {
    fn sys_open(&self, path: &str, mode: FileMode) -> Result<usize, &'static str>;
    fn sys_close(&self, fd: usize) -> Result<(), &'static str>;
    fn sys_read(&self, fd: usize, buffer: &mut[u8]) -> Result<usize, &'static str>;
    fn sys_write(&self, fd: usize, buffer: &[u8]) -> Result<usize, &'static str>;
    fn sys_fstat(&self, fd: usize) -> Result<FileStat, &'static str>;
    fn sys_mknod(&self, path: &str, major: i16, minor: i16) -> Result<(), &'static str>;
    fn sys_dup(&self, fd: usize) -> Result<usize, &'static str>;
    fn sys_pipe(&self) -> Result<(usize, usize), &'static str>;
    fn sys_dump_inode(&self);
}

// syscalls that are only exposed to the kernel
pub trait KernelVFS {
    // Save threadlocal objects to a temporary storage and return its id
    // For fdtable, only save the selected ones specified by `fds`
    fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE]) -> Result<usize, &'static str>;
    // Set threadlocal objects to a temporary object identify by the `id`
    fn sys_set_threadlocal(&self, id: usize) -> Result<(), &'static str>;
    // Tell the file system that this thread is exiting and thread local objects should be cleaned up
    fn sys_thread_exit(&self);
}

pub trait VFS: UsrVFS + KernelVFS {
    fn clone(&self) -> VFSPtr;
}

pub type VFSPtr = Box<dyn VFS + Send + Sync>;
pub type KernelVFSPtr = Box<dyn KernelVFS + Send + Sync>;