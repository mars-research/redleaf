//! RedLeaf input interface.
//!
//! ## Keyboard
//!
//! [Device] -- (1) -- [Driver] -- (2) -- [Translation Layer] -- (3) -- [Application]
//! 
//! (1) Variable-length scan codes, device/platform dependent
//! (2) Fixed-length `KeyCode`s (keydown, keyup) in `RawKeyboardEvent`s
//! (3) Characters, modifier keys, etc.

use rref::RRefVec;
use rref::traits::TypeIdentifiable;
use crate::error::Result;
use crate::rpc::RpcResult;

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
pub enum InputEvent {
    /// A raw keyboard event.
    RawKeyboardEvent(KeyCode),
}

#[derive(Copy, Clone, Debug)]
pub enum KeyCode {
    KeyDown(usize),
    KeyUp(usize),
}

impl TypeIdentifiable for InputEvent {
    fn type_id() -> u64 { 1 }
}

#[interface]
pub trait Input: Send + Sync {
    fn poll(&self, buffer: RRefVec<InputEvent>) -> RpcResult<Result<usize>>;
}
