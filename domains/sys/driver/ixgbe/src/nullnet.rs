use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::vec::Vec;
use rref::{RRefDeque};
use usr::rpc::RpcResult;
use usr::error::Result;
use crate::NetworkStats;

pub struct NullNet {}

impl NullNet {
    pub fn new() -> Self {
        Self{}
    }
}

impl usr::net::Net for NullNet {
    fn clone_net(&self) -> RpcResult<Box<dyn usr::net::Net>> {
        Ok(box Self::new())
    }

    fn submit_and_poll(&self, packets: &mut VecDeque<Vec<u8>
        >, collect: &mut VecDeque<Vec<u8>>, _tx: bool) -> RpcResult<Result<usize>> {

        let ret = packets.len();
        while let Some(pkt) = packets.pop_front() {
            collect.push_back(pkt);
        }
        Ok(Ok(ret))
    }

    fn submit_and_poll_rref(
        &self,
        mut packets: RRefDeque<[u8; 1514], 32>,
        mut collect: RRefDeque<[u8; 1514], 32>,
        _tx: bool,
        _pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>>
    {
        while let Some(pkt) = packets.pop_front() {
            collect.push_back(pkt);
        }

        Ok(Ok((collect.len(), packets, collect)))
    }

    fn poll(&self, _collect: &mut VecDeque<Vec<u8>>, _tx: bool) -> RpcResult<Result<usize>> {
        Ok(Ok(0))
    }

    fn poll_rref(&self, collect: RRefDeque<[u8; 1514], 512>, _tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        Ok(Ok((0, collect)))
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        Ok(Ok(NetworkStats::new()))
    }

    fn test_domain_crossing(&self) -> RpcResult<()> {
        Ok(())
    }
}
