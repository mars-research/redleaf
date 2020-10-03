/// Virtual file system interface
/// Implemented by xv6 file system
use alloc::boxed::Box;
use rref::RRefVec;

pub use crate::vfs::file::{FileMode, FileStat, INodeFileType};
pub use crate::vfs::directory::{DirectoryEntry, DirectoryEntryRef};
pub use crate::error::{Result, ErrorKind};
use crate::rpc::RpcResult;

pub mod file;
pub mod directory;

pub const NFILE: usize =       100;     // open files per system

// syscalls that are exposed to both the kernel and the users
pub trait UsrVFS: Send + Sync {
    fn sys_open(&self, path: &str, mode: FileMode) -> RpcResult<Result<usize>>;
    fn sys_close(&self, fd: usize) -> RpcResult<Result<()>>;
    fn sys_read(&self, fd: usize, buffer: &mut[u8]) -> RpcResult<Result<usize>>;
    fn sys_write(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_seek(&self, fd: usize, offset: usize) -> RpcResult<Result<()>>;
    fn sys_fstat(&self, fd: usize) -> RpcResult<Result<FileStat>>;
    fn sys_mknod(&self, path: &str, major: i16, minor: i16) -> RpcResult<Result<()>>;
    fn sys_dup(&self, fd: usize) -> RpcResult<Result<usize>>;
    fn sys_pipe(&self) -> RpcResult<Result<(usize, usize)>>;
    fn sys_mkdir(&self, path: &str) -> RpcResult<Result<()>>;
    fn sys_dump_inode(&self) -> RpcResult<Result<()>>;
}

// syscalls that are only exposed to the kernel
pub trait KernelVFS: Send + Sync  {
    // Save threadlocal objects to a temporary storage and return its id
    // For fdtable, only save the selected ones specified by `fds`
    fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE]) -> Result<usize>;
    // Set threadlocal objects to a temporary object identify by the `id`
    fn sys_set_threadlocal(&self, id: usize) -> Result<()>;
    // Tell the file system that this thread is exiting and thread local objects should be cleaned up
    fn sys_thread_exit(&self);
}

pub trait VFS: UsrVFS + KernelVFS + Send + Sync {
    fn clone(&self) -> Box<dyn VFS>;
}
