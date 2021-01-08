use rref::RRef;

use crate::rpc::RpcResult;

#[interface]
pub trait DomC {
    fn no_arg(&self) -> RpcResult<()>;
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>>;
}
