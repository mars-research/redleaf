#![no_std]

extern crate alloc;

pub mod sync;
pub mod syscalls;

pub use ::syscalls::errors;

