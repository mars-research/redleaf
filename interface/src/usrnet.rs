use alloc::boxed::Box;
use rref::RRefVec;

use crate::rpc::RpcResult;
use crate::error::Result;

#[interface]
/// UsrNet interface
pub trait UsrNet: Send + Sync {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>>;
    fn create(&self) -> RpcResult<Result<usize>>;
    fn listen(&self, socket: usize, port: u16) -> RpcResult<Result<()>>;
    fn poll(&self, tx: bool) -> RpcResult<Result<()>>;
    fn can_recv(&self, server: usize) -> RpcResult<Result<bool>>;
    fn is_listening(&self, server: usize) -> RpcResult<Result<bool>>;
    fn is_active(&self, socket: usize) -> RpcResult<Result<bool>>;
    fn close(&self, server: usize) -> RpcResult<Result<()>>;
    fn read_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    fn write_socket(&self, socket: usize, buffer: RRefVec<u8>, size: usize) -> RpcResult<Result<(usize, RRefVec<u8>)>>;
}
