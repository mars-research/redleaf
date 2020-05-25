#![no_std]
#![feature(trait_alias)]

extern crate alloc;
#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate bitflags;

pub mod bdev;
pub mod dom_a;
pub mod dom_c;
pub mod error;
pub mod net;
pub mod pci;
pub mod rpc;
pub mod vfs;
pub mod xv6;

extern crate red_idl;

