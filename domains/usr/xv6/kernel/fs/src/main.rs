#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(
    box_syntax,
    thread_local,
    untagged_unions
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate byteorder;

use alloc::boxed::Box;
use console::println;
use core::panic::PanicInfo;

use interface::rref::RRefVec;
use syscalls::{Heap, Syscall};
use sysfile::{FileMode, FileStat};
use interface::bdev::BDev;
use interface::rpc::RpcResult;
use interface::vfs::{KernelVFS, Result, UsrVFS, NFILE, VFS};

mod bcache;
mod block;
mod console_device;
mod cross_thread_temp_store;
mod cwd;
mod fs;
mod icache;
mod log;
mod net;
mod opened_file;
mod params;
mod pipe;
mod sysfile;

struct Rv6FS {}

impl Rv6FS {
    fn new() -> Self {
        Self {}
    }
}

impl VFS for Rv6FS {
    fn clone(&self) -> RpcResult<Box<dyn VFS>> {
        Ok(box Self {})
    }

    // KernelVFS part
    fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE]) -> RpcResult<Result<usize>> {
        Ok(sysfile::sys_save_threadlocal(fds))
    }
    fn sys_set_threadlocal(&self, id: usize) -> RpcResult<Result<()>> {
        Ok(sysfile::sys_set_threadlocal(id))
    }
    fn sys_thread_exit(&self) -> RpcResult<()> {
        Ok(sysfile::sys_thread_exit())
    }

    // UsrVFS part
    fn sys_open(
        &self,
        path: RRefVec<u8>,
        mode: FileMode,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        Ok((|| {
            let fd = sysfile::sys_open(core::str::from_utf8(path.as_slice())?, mode)?;
            Ok((fd, path))
        })())
    }
    fn sys_close(&self, fd: usize) -> RpcResult<Result<()>> {
        Ok(sysfile::sys_close(fd))
    }
    fn sys_read(
        &self,
        fd: usize,
        mut buffer: RRefVec<u8>,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        Ok((|| {
            let bytes_read = sysfile::sys_read(fd, buffer.as_mut_slice())?;
            Ok((bytes_read, buffer))
        })())
    }
    fn sys_write(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        Ok((|| {
            let bytes_read = sysfile::sys_write(fd, buffer.as_slice())?;
            Ok((bytes_read, buffer))
        })())
    }
    fn sys_seek(&self, fd: usize, offset: usize) -> RpcResult<Result<()>> {
        Ok(sysfile::sys_seek(fd, offset))
    }
    fn sys_fstat(&self, fd: usize) -> RpcResult<Result<FileStat>> {
        Ok(sysfile::sys_fstat(fd))
    }
    fn sys_mknod(&self, path: RRefVec<u8>, major: i16, minor: i16) -> RpcResult<Result<()>> {
        Ok((|| {
            let path = core::str::from_utf8(path.as_slice())?;
            sysfile::sys_mknod(path, major, minor)
        })())
    }
    fn sys_dup(&self, fd: usize) -> RpcResult<Result<usize>> {
        Ok(sysfile::sys_dup(fd))
    }
    fn sys_pipe(&self) -> RpcResult<Result<(usize, usize)>> {
        Ok(sysfile::sys_pipe())
    }
    fn sys_link(&self, old_path: RRefVec<u8>, new_path: RRefVec<u8>) -> RpcResult<Result<()>> {
        Ok((|| {
            let old_path = core::str::from_utf8(old_path.as_slice())?;
            let new_path = core::str::from_utf8(new_path.as_slice())?;
            sysfile::sys_link(&old_path, &new_path)
        })())
    }
    fn sys_unlink(&self, path: RRefVec<u8>) -> RpcResult<Result<()>> {
        Ok((|| {
            let path = core::str::from_utf8(path.as_slice())?;
            sysfile::sys_unlink(&path)
        })())
    }
    fn sys_mkdir(&self, path: RRefVec<u8>) -> RpcResult<Result<()>> {
        Ok((|| {
            let path = core::str::from_utf8(path.as_slice())?;
            sysfile::sys_mkdir(path)
        })())
    }
    fn sys_dump_inode(&self) -> RpcResult<Result<()>> {
        Ok(sysfile::sys_dump_inode())
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    bdev: Box<dyn BDev>,
) -> Box<dyn VFS> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    // libinterface::sysbdev::init(bdev);

    println!("init xv6 filesystem");
    fs::fsinit(params::ROOTDEV, bdev);
    println!("finish init xv6 filesystem");
    Box::new(Rv6FS::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6fs panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
