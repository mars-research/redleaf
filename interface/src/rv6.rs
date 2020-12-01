/// Rv6 system calls

use alloc::boxed::Box;
use rref::RRefVec;

use crate::vfs::{UsrVFS, NFILE};
use crate::net::Net;
use crate::usrnet::UsrNet;
use crate::bdev::NvmeBDev;
use crate::rpc::RpcResult;
use crate::tpm::UsrTpm;
pub use crate::vfs::{FileMode, FileStat};
pub use crate::error::{ErrorKind, Result};

#[interface]
pub trait Rv6: Send + Sync + UsrVFS + Net + UsrNet {
    fn clone(&self) -> RpcResult<Box<dyn Rv6>>;
    fn as_net(&self) -> RpcResult<Box<dyn Net>>;
    fn as_nvme(&self) -> RpcResult<Box<dyn NvmeBDev>>;
    fn as_usrnet(&self) -> RpcResult<Box<dyn UsrNet>>;
    fn get_usrnet(&self) -> RpcResult<Box<dyn UsrNet>>;
    fn get_usrtpm(&self) -> RpcResult<Box<dyn UsrTpm>>;
    fn sys_spawn_thread(&self, name: RRefVec<u8>, func: alloc::boxed::Box<dyn FnOnce() + Send>) -> RpcResult<Result<Box<dyn Thread>>>;
    // We need to pass a new instance of `rv6` as a parameter so that the proxy can be properly propagated.
    fn sys_spawn_domain(&self, rv6: Box<dyn Rv6>, path: RRefVec<u8>, args: RRefVec<u8>, fds: [Option<usize>; NFILE]) -> RpcResult<Result<Box<dyn Thread>>>;
    fn sys_getpid(&self) -> RpcResult<Result<u64>>;
    fn sys_uptime(&self) -> RpcResult<Result<u64>>;
    fn sys_sleep(&self, ns: u64) -> RpcResult<Result<()>>;
}

pub trait File: Send {
    fn read(&self, data: &mut [u8]) -> usize;
    fn write(&self, data: &[u8]) -> usize;
}

pub trait Thread: Send {
    fn join(&self) -> RpcResult<()>;
}
