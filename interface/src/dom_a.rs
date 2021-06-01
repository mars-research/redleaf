/// RedLeaf block device interface
use alloc::boxed::Box;
use crate::rref::{RRef, RRefDeque, Owned};
use crate::rpc::RpcResult;

pub struct OwnedTest {
    pub owned: Owned<u8>,
}

#[interface]
pub trait DomA {
    fn ping_pong(&self, buffer: RRef<[u8; 1024]>) -> RpcResult<RRef<[u8; 1024]>>;
    fn tx_submit_and_poll(&mut self,
        packets: RRefDeque<[u8; 100], 32>,
        reap_queue: RRefDeque<[u8; 100], 32>) -> RpcResult<(
            usize,
            RRefDeque<[u8; 100], 32>,
            RRefDeque<[u8; 100], 32>
        )>;
    fn test_owned(&self, rref: RRef<OwnedTest>) -> RpcResult<RRef<OwnedTest>>;
}
