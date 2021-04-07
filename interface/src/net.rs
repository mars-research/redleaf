/// RedLeaf network interface
use alloc::boxed::Box;
use crate::rref::{RRef, RRefDeque};
// TODO: remove once Ixgbe transitions to RRefDeque
use alloc::{vec::Vec, collections::VecDeque};
use crate::error::Result;
use crate::rpc::RpcResult;
use core::fmt;

pub struct NetworkStats {
    pub tx_count: u64,
    pub rx_count: u64,
    pub tx_dma_ok: u64,
    pub rx_dma_ok: u64,
    pub rx_missed: u64,
    pub rx_crc_err: u64
}

impl NetworkStats {
    pub fn new() -> Self {
        Self {
            tx_count: 0,
            rx_count: 0,
            tx_dma_ok: 0,
            rx_dma_ok: 0,
            rx_missed: 0,
            rx_crc_err: 0,
        }
    }

    pub fn stats_diff(&mut self, start: NetworkStats) {
        self.tx_count.saturating_sub(start.tx_count);
        self.rx_count.saturating_sub(start.rx_count);
        self.tx_dma_ok.saturating_sub(start.tx_dma_ok);
        self.rx_dma_ok.saturating_sub(start.rx_dma_ok);
        self.rx_missed.saturating_sub(start.rx_missed);
        self.rx_crc_err.saturating_sub(start.rx_crc_err);
    }
}

impl fmt::Display for NetworkStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "=> Tx stats: Count: {} dma_OK: {}\n", self.tx_count, self.tx_dma_ok);
        write!(f, "=> Rx stats: Count: {} dma_OK: {} missed: {} crc_err: {}", self.rx_count, self.rx_dma_ok, self.rx_missed, self.rx_crc_err)
    }
}

#[interface]
pub trait Net: Send + Sync {
    fn clone_net(&self) -> RpcResult<Box<dyn Net>>;

    fn submit_and_poll(&self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>>;

    // TODO: This is a non-rref interface for benchmark only and it needs to be clean up.
    fn poll(&self, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>>;

    fn submit_and_poll_rref(
        &self,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize) -> RpcResult<Result<(
            usize,
            RRefDeque<[u8; 1514], 32>,
            RRefDeque<[u8; 1514], 32>
        )>>;

    fn poll_rref(&self, collect: RRefDeque<[u8; 1514], 512>, tx: bool) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>>;

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>>;
    
    fn test_domain_crossing(&self) -> RpcResult<()>;
}
