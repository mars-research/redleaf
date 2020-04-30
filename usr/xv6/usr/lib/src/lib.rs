#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

pub mod syscalls;
pub mod io;

#[macro_use]
pub mod macros;

pub use syscalls::init;

