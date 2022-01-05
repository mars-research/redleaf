#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod io;
pub mod syscalls;

#[macro_use]
pub mod macros;

pub use crate::syscalls::init;

// BINARY LIBRARIES (THESE ARE LIBRARIES BECAUSE WE HAVE AN UNSAFE WRAPPER CALLING THEIR main() FUNCTION)
pub mod benchfs;
pub mod benchnet;
pub mod benchnvme;
pub mod dump_inode;
pub mod getpid;
pub mod httpd;
pub mod init;
pub mod ln;
pub mod ls;
pub mod mkdir;
pub mod rm;
pub mod rv6_testtpm;
pub mod sleep;
pub mod uptime;
pub mod wc;
