/// Xv6 system calls

use alloc::boxed::Box;

use crate::vfs::{UsrVFS, NFILE};
use crate::net::Net;
use crate::bdev::NvmeBDev;
use crate::rpc::RpcResult;
pub use crate::vfs::{FileMode, FileStat};
pub use crate::error::{ErrorKind, Result};

pub trait Xv6: Send + Sync + UsrVFS + Net {
    fn clone(&self) -> RpcResult<Box<dyn Xv6>>;
    fn as_net(&self) -> RpcResult<Box<dyn Net>>;
    fn as_nvme(&self) -> RpcResult<Box<dyn NvmeBDev>>;
    fn sys_spawn_thread(&self, name: &str, func: alloc::boxed::Box<dyn FnOnce() + Send>) -> RpcResult<Box<dyn Thread>>;
    // We need to pass a new instance of `rv6` as a parameter so that the proxy can be properly propagated.
    fn sys_spawn_domain(&self, rv6: Box<dyn Xv6>, path: &str, args: &str, fds: [Option<usize>; NFILE]) -> RpcResult<Result<Box<dyn Thread>>>;
    fn sys_getpid(&self) -> RpcResult<Result<u64>>;
    fn sys_uptime(&self) -> RpcResult<Result<u64>>;
}

pub trait File: Send {
    fn read(&self, data: &mut [u8]) -> usize;
    fn write(&self, data: &[u8]) -> usize;
}

pub trait Thread: Send {
    fn join(&self) -> RpcResult<()>;
}
