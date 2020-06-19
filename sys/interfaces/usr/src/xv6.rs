/// Xv6 system calls

use alloc::boxed::Box;

use crate::vfs::{UsrVFS, NFILE};
use crate::net::Net;
use crate::bdev::NvmeBDev;
pub use crate::vfs::{FileMode, FileStat};
pub use crate::error::{ErrorKind, Result};

pub trait Xv6: Send + Sync + UsrVFS + Net {
    fn clone(&self) -> Box<dyn Xv6>;
    fn as_net(&self) -> Box<dyn Net>;
    fn as_nvme(&self) -> Box<dyn NvmeBDev>;
    fn sys_spawn_thread(&self, name: &str, func: alloc::boxed::Box<dyn FnOnce() + Send>) -> Box<dyn Thread>;
    // We need to pass a new instance of `rv6` as a parameter so that the proxy can be properly propagated.
    fn sys_spawn_domain(&self, rv6: Box<dyn Xv6>, path: &str, args: &str, fds: [Option<usize>; NFILE]) -> Result<Box<dyn Thread>>;
    fn sys_rdtsc(&self) -> u64;
}

pub trait File: Send {
    fn read(&self, data: &mut [u8]) -> usize;
    fn write(&self, data: &[u8]) -> usize;
}

pub trait Thread: Send {
    fn join(&self);
}
