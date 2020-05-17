/// RedLeaf block device interface
use rref::RRef;
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

