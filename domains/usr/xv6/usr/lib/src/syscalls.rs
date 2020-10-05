use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Once;
use usr_interface::vfs::NFILE;
use usr_interface::xv6::{FileMode, FileStat, Result, Thread, Xv6};
use rref::RRefVec;

static SYSCALL: Once<Box<dyn Xv6>> = Once::new();

pub fn init(s: Box<dyn Xv6>) {
    SYSCALL.call_once(|| s);
}

pub fn sys_spawn_domain(path: &str, args: &str, fds: &[Option<usize>]) -> Result<Box<dyn Thread>> {
    assert!(fds.len() <= NFILE);
    let mut arr: [Option<usize>; NFILE] = array_init::array_init(|_| None);
    arr[..fds.len()].clone_from_slice(&fds);
    let rv6 = &**SYSCALL.r#try().unwrap();
    rv6.sys_spawn_domain(rv6.clone()?, path, args, arr)?
}

pub fn sys_getpid() -> Result<u64> {
    SYSCALL.r#try().unwrap().sys_getpid()?   
}

pub fn sys_uptime() -> Result<u64> {
    SYSCALL.r#try().unwrap().sys_uptime()?   
}

pub fn sys_sleep(ns: u64) -> Result<()> {
    SYSCALL.r#try().unwrap().sys_sleep(ns)?      
}

pub fn sys_open(path: &str, mode: FileMode) -> Result<usize> {
    SYSCALL.r#try().unwrap().sys_open(path, mode)?
}

pub fn sys_close(fd: usize) -> Result<()> {
    SYSCALL.r#try().unwrap().sys_close(fd)?
}

// See comment for `sys_write_slice_slow`
pub fn sys_read_slice_slow(fd: usize, buffer: &mut [u8]) -> Result<usize> {
    let vec = RRefVec::from_slice(buffer);
    let (size, vec) = sys_write(fd, vec)?;
    for (dest, src) in buffer.iter_mut().zip(vec.as_slice()) {
        *dest = *src;
    }
    Ok(size)
}

pub fn sys_read(fd: usize, buffer: RRefVec<u8>) -> Result<(usize, RRefVec<u8>)> {
    SYSCALL.r#try().unwrap().sys_write(fd, buffer)?
}

// Implicitly convert the slice to a RRefVec.
// Slower than `sys_write` but good for prototyping
pub fn sys_write_slice_slow(fd: usize, buffer: &[u8]) -> Result<usize> {
    let buffer = RRefVec::from_slice(buffer);
    let (size, _buffer) = sys_write(fd, buffer)?;
    Ok(size)
}

pub fn sys_write(fd: usize, buffer: RRefVec<u8>) -> Result<(usize, RRefVec<u8>)> {
    SYSCALL.r#try().unwrap().sys_write(fd, buffer)?
}

pub fn sys_fstat(fd: usize) -> Result<FileStat> {
    SYSCALL.r#try().unwrap().sys_fstat(fd)?
}

pub fn sys_mknod(path: &str, major: i16, minor: i16) -> Result<()> {
    SYSCALL.r#try().unwrap().sys_mknod(path, major, minor)?
}

pub fn sys_dup(fd: usize) -> Result<usize> {
    SYSCALL.r#try().unwrap().sys_dup(fd)?
}

pub fn sys_pipe() -> Result<(usize, usize)> {
    SYSCALL.r#try().unwrap().sys_pipe()?
}

pub fn sys_mkdir(path: &str) -> Result<()> {
    SYSCALL.r#try().unwrap().sys_mkdir(path)?
}

pub fn sys_dump_inode() -> Result<()> {
    SYSCALL.r#try().unwrap().sys_dump_inode()?
}
