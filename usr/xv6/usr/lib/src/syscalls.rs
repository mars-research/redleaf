use spin::Once;
use alloc::boxed::Box;
use alloc::vec::Vec;
use usr_interface::xv6::{Thread, Xv6, FileMode, FileStat};
use usr_interface::vfs::NFILE;

static SYSCALL: Once<Box<dyn Xv6 + Send + Sync>> = Once::new();

pub fn init(s: Box<dyn Xv6 + Send + Sync>) {
    SYSCALL.call_once(|| s);
}

pub fn sys_spawn_domain(path: &str, args: &str, fds: &[Option<usize>]) -> Result<Box<dyn Thread>, &'static str> {
    if fds.len() > NFILE {
        return Err("fds too long");
    }
    let mut arr: [Option<usize>; NFILE] = array_init::array_init(|_| None);
    arr[..fds.len()].clone_from_slice(&fds);
    SYSCALL.r#try().unwrap().sys_spawn_domain(path, args, arr)
}


pub fn sys_open(path: &str, mode: FileMode) -> Result<usize, &'static str> {
    SYSCALL.r#try().unwrap().sys_open(path, mode)
}

pub fn sys_close(fd: usize) -> Result<(), &'static str> {
    SYSCALL.r#try().unwrap().sys_close(fd)
}

pub fn sys_read(fd: usize, buffer: &mut[u8]) -> Result<usize, &'static str> {
    SYSCALL.r#try().unwrap().sys_read(fd, buffer)
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> Result<usize, &'static str> {
    SYSCALL.r#try().unwrap().sys_write(fd, buffer)
}

pub fn sys_fstat(fd: usize) -> Result<FileStat, &'static str> {
    SYSCALL.r#try().unwrap().sys_fstat(fd)
}

pub fn sys_mknod(path: &str, major: i16, minor: i16) -> Result<(), &'static str> {
    SYSCALL.r#try().unwrap().sys_mknod(path, major, minor)
}

pub fn sys_dup(fd: usize) -> Result<usize, &'static str> {
    SYSCALL.r#try().unwrap().sys_dup(fd)
}

pub fn sys_pipe() -> Result<(usize, usize), &'static str>{
    SYSCALL.r#try().unwrap().sys_pipe()
}
