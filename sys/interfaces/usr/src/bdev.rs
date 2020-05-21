/// RedLeaf block device interface
use rref::{RRef, RRefDeque};
use syscalls::errors::Result;

use crate::rpc::RpcResult;

pub const BSIZE: usize =        4096;   // block size

pub trait BDev: Send + Sync {
    fn read(&self, block: u32, data: RRef<[u8; BSIZE]>) -> RpcResult<RRef<[u8; BSIZE]>>;
    fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>) -> RpcResult<()>;
}

// pub trait SyncBDev {
//     fn read(&self, block: u32, data: &mut [u8]);
//     fn write(&self, block: u32, data: &[u8]);
// }

// pub trait AsyncBDev {
//     fn submit(&self, block: u64, write: bool, buf: Box<[u8]>) -> Result<u32>;
//     fn poll(&self, slot: u32) -> Result<Option<Box<[u8]>>>;
// }

// pub trait BDev: SyncBDev + AsyncBDev {}

pub struct BlkReq {
   pub data: [u8; 4096],
   pub data_len: usize,
   pub block: u64,
}

impl BlkReq {
    pub fn new() -> Self {
        Self {
            data: [0u8; 4096],
            data_len: 4096,
            block: 0,
        }
    }

    pub fn from_data(data: [u8; 4096]) -> Self {
        Self {
            data,
            data_len: 4096,
            block: 0,
        }
    }

}

pub trait NvmeBDev : Send {
    fn submit_and_poll_rref(
        &self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        write: bool,
        ) -> (
            usize,
            RRefDeque<BlkReq, 128>,
            RRefDeque<BlkReq, 128>,
        );

    fn poll_rref(&mut self, collect: RRefDeque<BlkReq, 1024>) ->
            (usize, RRefDeque<BlkReq, 1024>);
}
