#![no_std]
#![crate_name = "pcid"]
#![crate_type = "lib"]

#![feature(
    llvm_asm,
)]

#[macro_use] extern crate bitflags;
extern crate byteorder;
#[macro_use] extern crate serde_derive;

#[macro_use]
extern crate alloc;

mod bar;
mod bus;
mod class;
mod dev;
mod func;
pub mod header;
pub mod pci;

