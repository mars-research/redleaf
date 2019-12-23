use crate::syscalls::{Syscall, FileMode};

pub fn ls(s: &Syscall, path: &str) {
    (s.sys_println)(&format!("{:?}", (s.sys_open)("/", FileMode::Read)));
}