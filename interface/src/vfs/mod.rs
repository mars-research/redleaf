/// Virtual file system interface
/// Implemented by xv6 file system
/// Some of the syscalls do no return the buffer back to the caller. Feel free
/// to change it if it's needed.
use alloc::boxed::Box;
use crate::rref::RRefVec;

pub use crate::vfs::file::{FileMode, FileStat, INodeFileType};
pub use crate::vfs::directory::{DirectoryEntry, DirectoryEntryRef};
pub use crate::error::{Result, ErrorKind};
use crate::rpc::RpcResult;

pub mod file;
pub mod directory;

pub const NFILE: usize =       100;     // open files per system

// syscalls that are exposed to both the kernel and the users
#[interface]
pub trait UsrVFS: Send + Sync {
    fn sys_open(&self, path: RRefVec<u8>, mode: FileMode) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_close(&self, fd: usize) -> RpcResult<Result<()>>;
    fn sys_read(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_write(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_seek(&self, fd: usize, offset: usize) -> RpcResult<Result<()>>;
    fn sys_fstat(&self, fd: usize) -> RpcResult<Result<FileStat>>;
    fn sys_mknod(&self, path: RRefVec<u8>, major: i16, minor: i16) -> RpcResult<Result<()>>;
    fn sys_dup(&self, fd: usize) -> RpcResult<Result<usize>>;
    fn sys_pipe(&self) -> RpcResult<Result<(usize, usize)>>;
    fn sys_link(&self, old_path: RRefVec<u8>, new_path: RRefVec<u8>) -> RpcResult<Result<()>>;
    fn sys_unlink(&self, path: RRefVec<u8>) -> RpcResult<Result<()>>;
    fn sys_mkdir(&self, path: RRefVec<u8>) -> RpcResult<Result<()>>;
    fn sys_dump_inode(&self) -> RpcResult<Result<()>>;
}

// syscalls that are only exposed to the kernel
pub trait KernelVFS: Send + Sync  {
    // Save threadlocal objects to a temporary storage and return its id
    // For fdtable, only save the selected ones specified by `fds`
    fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE]) -> RpcResult<Result<usize>>;
    // Set threadlocal objects to a temporary object identify by the `id`
    fn sys_set_threadlocal(&self, id: usize) -> RpcResult<Result<()>>;
    // Tell the file system that this thread is exiting and thread local objects should be cleaned up
    fn sys_thread_exit(&self) -> RpcResult<()>;
}

/// Super trait of UsrVFS and KernelVFS.
/// Since super trait is not supported by ngc, we copied and pasted UsrVFS and KernelVFS in here.
#[interface]
pub trait VFS: Send + Sync {
    // UserVFS starts.
    fn sys_open(&self, path: RRefVec<u8>, mode: FileMode) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_close(&self, fd: usize) -> RpcResult<Result<()>>;
    fn sys_read(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_write(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn sys_seek(&self, fd: usize, offset: usize) -> RpcResult<Result<()>>;
    fn sys_fstat(&self, fd: usize) -> RpcResult<Result<FileStat>>;
    fn sys_mknod(&self, path: RRefVec<u8>, major: i16, minor: i16) -> RpcResult<Result<()>>;
    fn sys_dup(&self, fd: usize) -> RpcResult<Result<usize>>;
    fn sys_pipe(&self) -> RpcResult<Result<(usize, usize)>>;
    fn sys_link(&self, old_path: RRefVec<u8>, new_path: RRefVec<u8>) -> RpcResult<Result<()>>;
    fn sys_unlink(&self, path: RRefVec<u8>) -> RpcResult<Result<()>>;
    fn sys_mkdir(&self, path: RRefVec<u8>) -> RpcResult<Result<()>>;
    fn sys_dump_inode(&self) -> RpcResult<Result<()>>;

    // KernelVFS starts.
    // Save threadlocal objects to a temporary storage and return its id
    // For fdtable, only save the selected ones specified by `fds`
    fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE]) -> RpcResult<Result<usize>>;
    // Set threadlocal objects to a temporary object identify by the `id`
    fn sys_set_threadlocal(&self, id: usize) -> RpcResult<Result<()>>;
    // Tell the file system that this thread is exiting and thread local objects should be cleaned up
    fn sys_thread_exit(&self) -> RpcResult<()>;

    fn clone(&self) -> RpcResult<Box<dyn VFS>>;
}
