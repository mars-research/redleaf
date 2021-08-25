use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use interface::error::Result;
use interface::net::NetworkStats;
use interface::rpc::RpcResult;
use interface::rref::RRefDeque;

pub struct NullNet {}

impl NullNet {
    pub fn new() -> Self {
        Self {}
    }
}

impl interface::net::Net for NullNet {
    fn clone_net(&self) -> RpcResult<Box<dyn interface::net::Net>> {
        Ok(box Self::new())
    }

    fn submit_and_poll(
        &self,
        packets: &mut VecDeque<Vec<u8>>,
        collect: &mut VecDeque<Vec<u8>>,
        _tx: bool,
    ) -> RpcResult<Result<usize>> {
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
        _pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>, RRefDeque<[u8; 1514], 32>)>> {
        while let Some(pkt) = packets.pop_front() {
            collect.push_back(pkt);
        }

        Ok(Ok((collect.len(), packets, collect)))
    }

    fn poll(&self, _collect: &mut VecDeque<Vec<u8>>, _tx: bool) -> RpcResult<Result<usize>> {
        Ok(Ok(0))
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<[u8; 1514], 512>,
        _tx: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        Ok(Ok((0, collect)))
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        Ok(Ok(NetworkStats::new()))
    }

    fn test_domain_crossing(&self) -> RpcResult<()> {
        Ok(())
    }
}
