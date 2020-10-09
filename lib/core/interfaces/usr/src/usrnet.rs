use alloc::boxed::Box;
use rref::RRefVec;

use crate::rpc::RpcResult;
use crate::error::Result;

/// UsrNet interface
pub trait UsrNet: Send + Sync {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>>;
    fn listen(&self, port: u16) -> RpcResult<Result<usize>>;
    fn is_active(&self, server: usize) -> RpcResult<Result<bool>>;
    fn close(&self, server: usize) -> RpcResult<Result<()>>;
    fn read_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn write_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
}
