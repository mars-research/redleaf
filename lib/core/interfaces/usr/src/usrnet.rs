/// Xv6 system calls

use alloc::boxed::Box;

use crate::rpc::RpcResult;
pub use crate::error::{ErrorKind, Result};

pub trait UsrNet: Send + Sync {
    fn clone(&self) -> RpcResult<Box<dyn UsrNet>>;
}
