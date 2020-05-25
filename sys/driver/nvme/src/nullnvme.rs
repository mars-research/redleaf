use rref::{RRef, RRefDeque};
use usr::rpc::RpcResult;
use usr::error::Result;
use usr::bdev::{BlkReq, NvmeBDev};

pub struct NullNvme {}

impl NullNvme {
    pub fn new() -> Self {
        Self{}
    }
}

impl NvmeBDev for NullNvme {
    fn submit_and_poll_rref(
        &self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        write: bool,
        ) -> RpcResult<Result<(
            usize,
            RRefDeque<BlkReq, 128>,
            RRefDeque<BlkReq, 128>,
        )>> {
        Ok(Ok((submit.len(), collect, submit)))
    }

    fn poll_rref(&mut self, collect: RRefDeque<BlkReq, 1024>) ->
            RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>> {
        Ok(Ok((collect.len(), collect)))
    }

    fn get_stats(&mut self) -> RpcResult<Result<(u64, u64)>> {
        Ok(Ok((0, 0)))
    }
}
