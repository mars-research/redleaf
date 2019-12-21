use crate::capabilities::Capability;

#[derive(PartialEq, Clone, Copy)]
pub enum FileMode {
    Read,
    Write,
    ReadWrite,
    Create
}

impl FileMode {
    pub fn readable(self) -> bool {
        self == FileMode::Read
    }

    pub fn writeable(self) -> bool {
        self == FileMode::ReadWrite || self == FileMode::Write
    }
}

#[derive(Copy, Clone)]
pub struct Syscall {
    pub sys_print: fn(s: &str),
    pub sys_yield: fn(),
    pub sys_create_thread: fn(name: &str, func: extern fn()) -> Capability,
    pub sys_open: fn(path: &str, mode: FileMode) -> Option<usize>,
    pub init_fs_temp: fn(),
}
