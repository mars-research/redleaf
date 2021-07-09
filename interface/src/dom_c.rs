use crate::rref::RRef;

use crate::rpc::RpcResult;

use alloc::boxed::Box;

#[interface]
pub trait DomC {
    fn no_arg(&self) -> RpcResult<()>;
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>>;
    fn init_dom_c(&self, c: Box<dyn DomC>) -> RpcResult<()>;
}
