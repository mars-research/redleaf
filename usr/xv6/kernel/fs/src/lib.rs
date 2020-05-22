#![no_std]
// #![no_builtins]
#![forbid(unsafe_code)]
#![feature(abi_x86_interrupt)]
#![feature(
    asm,
    allocator_api,
    alloc_layout_extra,
    alloc_error_handler,
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    thread_local,
    untagged_unions,
    panic_info_message,
    ptr_wrapping_offset_from,
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
extern crate spin;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate byteorder;
extern crate memcpy;
extern crate syscalls;
extern crate tls;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use console::println;
use core::panic::PanicInfo;

use libtime::get_rdtsc;
use rref;
use sysfile::{FileMode, FileStat};
use syscalls::{Syscall, Heap};
use usr_interface::vfs::{UsrVFS, KernelVFS, VFS, NFILE, Result};
use usr_interface::bdev::BDev;

mod bcache;
mod block;
mod console_device;
mod cross_thread_temp_store;
mod cwd;
mod fs;
mod icache;
mod log;
mod opened_file;
mod params;
mod pipe;
mod sysfile;

struct Rv6FS {}

impl Rv6FS {
    fn new() -> Self {
        Self{}
    }
}

impl VFS for Rv6FS {
    fn clone(&self) -> Box<dyn VFS> {
        box Self{}
    }
}

impl KernelVFS for Rv6FS {
    fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE]) -> Result<usize> {
        sysfile::sys_save_threadlocal(fds)
    }
    fn sys_set_threadlocal(&self, id: usize) -> Result<()> {
        sysfile::sys_set_threadlocal(id)
    }
    fn sys_thread_exit(&self) {
        sysfile::sys_thread_exit()
    }
}

impl UsrVFS for Rv6FS {
    fn sys_open(&self, path: &str, mode: FileMode) -> Result<usize> {
        sysfile::sys_open(path, mode)
    }
    fn sys_close(&self, fd: usize) -> Result<()> {
        sysfile::sys_close(fd)
    }
    fn sys_read(&self, fd: usize, buffer: &mut[u8]) -> Result<usize> {
        sysfile::sys_read(fd, buffer)
    }
    fn sys_write(&self, fd: usize, buffer: &[u8]) -> Result<usize> {
        sysfile::sys_write(fd, buffer)
    }
    fn sys_seek(&self, fd: usize, offset: usize) -> Result<()> {
        sysfile::sys_seek(fd, offset)
    }
    fn sys_fstat(&self, fd: usize) -> Result<FileStat> {
        sysfile::sys_fstat(fd)
    }
    fn sys_mknod(&self, path: &str, major: i16, minor: i16) -> Result<()> {
        sysfile::sys_mknod(path, major, minor)
    }
    fn sys_dup(&self, fd: usize) -> Result<usize> {
        sysfile::sys_dup(fd)
    }
    fn sys_pipe(&self) -> Result<(usize, usize)> {
        sysfile::sys_pipe()
    }
    fn sys_dump_inode(&self) {
        let inode = icache::ICACHE.lock().get(params::ROOTDEV, params::ROOTINO).unwrap();
        inode.lock().print(&mut log::LOG.r#try().unwrap().begin_transaction(), 0);
    }
}


#[no_mangle]
pub fn init(s: Box<dyn Syscall + Send + Sync>,
            heap: Box<dyn Heap + Send + Sync>,
            bdev: Box<dyn BDev>) -> Box<dyn VFS> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    // libusr::sysbdev::init(bdev);

    println!("init xv6 filesystem");
    fs::fsinit(params::ROOTDEV, bdev);
    println!("finish init xv6 filesystem");
    Box::new(Rv6FS::new()) 
}

// fn fs_benchmark(buf_size: usize, path: &str) {
//     let start = get_rdtsc();
//     let fd = sysfile::sys_open(path, FileMode::READ).unwrap();
//     let mut buff = Vec::new();
//     buff.resize(buf_size, 0 as u8);
//     let mut bytes_read = 0;
//     while let Ok(sz) = sysfile::sys_read(fd, buff.as_mut_slice()) {
//         bytes_read += sz;
//         if sz < 512 {
//             break;
//         }
//     }
//     sysfile::sys_close(fd).unwrap();
//     let end = get_rdtsc();
//     println!("we read {} bytes at a time, in total {} bytes from {} using {} cycles", buf_size, bytes_read, path, end - start);
// }

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6fs panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
