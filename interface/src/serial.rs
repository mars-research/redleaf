use crate::input::InputEvent;
use rref::RRefVec;
use crate::error::Result;
use crate::rpc::RpcResult;

#[interface]
pub trait Serial: Send + Sync {
    fn read(&self, buffer: RRefVec<InputEvent>) -> RpcResult<Result<(RRefVec<InputEvent>, usize)>>;
    fn write(&self, buffer: RRefVec<InputEvent>) -> RpcResult<Result<(RRefVec<InputEvent>, usize)>>;
}
