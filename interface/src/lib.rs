#![no_std]
#![feature(trait_alias)]
#![allow(incomplete_features)]
#![feature(const_generics)]
#![feature(negative_impls)]
#![feature(auto_traits)]
#![feature(specialization)]
#![feature(type_name_of_val)]
#![feature(core_panic)]

// Features needed for proxy
#![feature(global_asm, type_ascription)]

// Features that we need because of cargo expand
#![feature(core_intrinsics, fmt_internals, derive_clone_copy, derive_eq, structural_match)]

extern crate alloc;
#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate interface_attribute_placeholder;

pub mod bdev;
pub mod dom_c;
pub mod error;
pub mod net;
pub mod pci;
pub mod rpc;
pub mod usrnet;
pub mod vfs;
pub mod rv6;
pub mod tpm;
pub mod rref;
pub mod typeid;
pub mod example;

pub mod proxy;

pub mod domain_create;
