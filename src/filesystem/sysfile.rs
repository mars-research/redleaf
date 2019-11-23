//
// File-system system calls.
// Arguments are checked
//
use alloc::boxed::Box;
use core::cell::RefCell;

use crate::filesystem::fcntl::{O_RDONLY, O_WRONLY, O_RDWR, O_CREATE};
use crate::filesystem::file::File;
use crate::filesystem::params;

// Opened files of the current thread. Use file descriptors to index into this array.
// I put it in a thread_local variable in fs instead of a member variable of `Thread`
// because this helps isolating the two modules.
#[thread_local]
static FD_TABLE: RefCell<[Option<Box<File>>; params::NOFILE]> = RefCell::new(
    [None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None]
);

pub fn sys_open(path: &str, omode: u32) {
    
}

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn fdalloc(f: Box<File>) -> Option<usize> {
    FD_TABLE
        .borrow()
        .iter()
        .position(|f| f.is_none())
        .map(|fd| {
            FD_TABLE.borrow_mut()[fd].replace(f);
            fd
        })
}
