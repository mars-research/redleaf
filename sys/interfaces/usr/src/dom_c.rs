use rref::RRef;

use crate::rpc::RpcResult;

pub trait DomC {
    fn no_arg(&self);
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RRef<usize>;
}
