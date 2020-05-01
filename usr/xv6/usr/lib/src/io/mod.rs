use core::fmt;

mod error;

use crate::syscalls::sys_write;

use error::{ErrorKind, Result};


pub static STDIN_FD: usize = 0;
pub static STDOUT_FD: usize = 1;
pub static STDERR_FD: usize = 2;

pub fn write_fmt(fd: usize, fmt: fmt::Arguments<'_>) -> Result<()> {
    // Create a shim which translates a Write to a fmt::Write and saves
    // off I/O errors. instead of discarding them
    struct Adaptor {
        fd: usize,
        error: Result<()>,
    }

    impl fmt::Write for Adaptor {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // match sys_write(self.fd, s.as_bytes()) {
            //     Ok(()) => Ok(()),
            //     Err(e) => {
            //         self.error = Err(e);
            //         Err(fmt::Error)
            //     }
            // }
            sys_write(self.fd, s.as_bytes()).unwrap();
            Ok(())
        }
    }

    let mut output = Adaptor { fd, error: Ok(()) };
    match fmt::write(&mut output, fmt) {
        Ok(()) => Ok(()),
        Err(..) => {
            // check if the error came from the underlying `Write` or not
            if output.error.is_err() {
                output.error
            } else {
                Err(ErrorKind::FormatError)
            }
        }
    }
}

pub fn _print(args: fmt::Arguments<'_>) {
    write_fmt(STDOUT_FD, args);
}

pub fn _eprint(args: fmt::Arguments<'_>) {
    write_fmt(STDERR_FD, args);
}