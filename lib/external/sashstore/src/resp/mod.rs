//! This module is an adapted form (for no-std, unsafe compatibility) of the resp crate v1.0.2:
//! https://github.com/iorust/resp written by Qing Yan <admin@zensh.com>
//!
//! resp is released under license: MIT/Apache-2.0
pub mod serialize;
pub mod value;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum DecodeError {
    InvalidInput,
    InvalidType,
    InvalidData,
    UnexpectedEof,
}
