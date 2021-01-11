/// Syscall helpers for rv6 user domains.
///
/// Some syscalls offers `sys_xxx_slice_slow` variants that converts the
/// &str arguments to RRefVec<u8>. They are Slower than the `sys_xxx` variants
/// but they are easier to use and good for prototyping.
use alloc::boxed::Box;
use alloc::vec::Vec;
use rref::RRefVec;
use spin::Once;
use usr_interface::rv6::{FileMode, FileStat, Result, Rv6, Thread};
use usr_interface::vfs::{UsrVFS, NFILE};

static SYSCALL: Once<Box<dyn Rv6>> = Once::new();
static FS: Once<Box<dyn UsrVFS>> = Once::new();

pub fn init(s: Box<dyn Rv6>) {
    let fs = s.as_vfs().unwrap();
    FS.call_once(|| fs);
    SYSCALL.call_once(|| s);
}

pub fn sys_spawn_domain_slice_slow(
    path: &str,
    args: &str,
    fds: &[Option<usize>],
) -> Result<Box<dyn Thread>> {
    sys_spawn_domain(
        RRefVec::from_slice(path.as_bytes()),
        RRefVec::from_slice(args.as_bytes()),
        fds,
    )
}

pub fn sys_spawn_domain(
    path: RRefVec<u8>,
    args: RRefVec<u8>,
    fds: &[Option<usize>],
) -> Result<Box<dyn Thread>> {
    assert!(fds.len() <= NFILE);
    let mut arr: [Option<usize>; NFILE] = array_init::array_init(|_| None);
    arr[..fds.len()].clone_from_slice(&fds);
    let rv6 = &**SYSCALL.r#try().unwrap();
    rv6.sys_spawn_domain(rv6.clone_rv6()?, path, args, arr)?
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

pub fn sys_open_slice_slow(path: &str, mode: FileMode) -> Result<usize> {
    let (fd, _) = sys_open(RRefVec::from_slice(path.as_bytes()), mode)?;
    Ok(fd)
}

pub fn sys_open(path: RRefVec<u8>, mode: FileMode) -> Result<(usize, RRefVec<u8>)> {
    FS.r#try().unwrap().sys_open(path, mode)?
}

pub fn sys_close(fd: usize) -> Result<()> {
    FS.r#try().unwrap().sys_close(fd)?
}

// See comment for `sys_write_slice_slow`
pub fn sys_read_slice_slow(fd: usize, buffer: &mut [u8]) -> Result<usize> {
    let vec = RRefVec::from_slice(buffer);
    let (size, vec) = sys_read(fd, vec)?;
    for (dest, src) in buffer.iter_mut().zip(vec.as_slice()) {
        *dest = *src;
    }
    Ok(size)
}

pub fn sys_read(fd: usize, buffer: RRefVec<u8>) -> Result<(usize, RRefVec<u8>)> {
    FS.r#try().unwrap().sys_read(fd, buffer)?
}

pub fn sys_write_slice_slow(fd: usize, buffer: &[u8]) -> Result<usize> {
    let buffer = RRefVec::from_slice(buffer);
    let (size, _buffer) = sys_write(fd, buffer)?;
    Ok(size)
}

pub fn sys_write(fd: usize, buffer: RRefVec<u8>) -> Result<(usize, RRefVec<u8>)> {
    FS.r#try().unwrap().sys_write(fd, buffer)?
}

pub fn sys_fstat(fd: usize) -> Result<FileStat> {
    FS.r#try().unwrap().sys_fstat(fd)?
}

pub fn sys_mknod_slice_slow(path: &str, major: i16, minor: i16) -> Result<()> {
    sys_mknod(RRefVec::from_slice(path.as_bytes()), major, minor)
}

pub fn sys_mknod(path: RRefVec<u8>, major: i16, minor: i16) -> Result<()> {
    FS.r#try().unwrap().sys_mknod(path, major, minor)?
}

pub fn sys_dup(fd: usize) -> Result<usize> {
    FS.r#try().unwrap().sys_dup(fd)?
}

pub fn sys_pipe() -> Result<(usize, usize)> {
    FS.r#try().unwrap().sys_pipe()?
}

pub fn sys_link_slice_slow(old_path: &str, new_path: &str) -> Result<()> {
    sys_link(
        RRefVec::from_slice(old_path.as_bytes()),
        RRefVec::from_slice(new_path.as_bytes()),
    )
}

pub fn sys_link(old_path: RRefVec<u8>, new_path: RRefVec<u8>) -> Result<()> {
    FS.r#try().unwrap().sys_link(old_path, new_path)?
}

pub fn sys_unlink_slice_slow(path: &str) -> Result<()> {
    sys_unlink(RRefVec::from_slice(path.as_bytes()))
}

pub fn sys_unlink(path: RRefVec<u8>) -> Result<()> {
    FS.r#try().unwrap().sys_unlink(path)?
}

pub fn sys_mkdir_slice_slow(path: &str) -> Result<()> {
    sys_mkdir(RRefVec::from_slice(path.as_bytes()))
}

pub fn sys_mkdir(path: RRefVec<u8>) -> Result<()> {
    FS.r#try().unwrap().sys_mkdir(path)?
}

pub fn sys_seek(fs: usize, offset: usize) -> Result<()> {
    FS.r#try().unwrap().sys_seek(fs, offset)?
}

pub fn sys_dump_inode() -> Result<()> {
    FS.r#try().unwrap().sys_dump_inode()?
}
