//
// File-system system calls.
// Arguments are checked
//
use alloc::boxed::Box;

use crate::filesystem::file::File;
use crate::thread;

pub fn sys_open() {

}

// Allocate a file descriptor for the given file.
// Takes over file reference from caller on success.
fn fdalloc(f: Box<File>) -> Option<usize> {
    let mut myproc = thread::CURRENT.borrow_mut();
    let myproc = myproc.as_mut().unwrap();
    myproc.files
        .iter()
        .position(|f| f.is_none())
        .map(|fd| {
            myproc.files[fd].replace(f);
            fd
        })
}
