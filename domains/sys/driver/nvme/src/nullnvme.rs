use rref::RRefDeque;
use interface::bdev::{BlkReq, NvmeBDev};
use interface::error::Result;
use interface::rpc::RpcResult;

pub struct NullNvme {}

impl NullNvme {
    pub fn new() -> Self {
        Self {}
    }
}

impl NvmeBDev for NullNvme {
    fn submit_and_poll_rref(
        &self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        _write: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)>> {
        Ok(Ok((submit.len(), collect, submit)))
    }

    fn poll_rref(&self, collect: RRefDeque<BlkReq, 1024>) ->
            RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>> {
        Ok(Ok((collect.len(), collect)))
    }

    fn get_stats(&self) -> RpcResult<Result<(u64, u64)>> {
        Ok(Ok((0, 0)))
    }
}
