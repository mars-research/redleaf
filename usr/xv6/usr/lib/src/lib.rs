#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod io;
pub mod syscalls;

#[macro_use]
pub mod macros;

pub use syscalls::init;
