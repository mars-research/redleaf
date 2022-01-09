#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra,
    assoc_char_funcs
)]

extern crate alloc;
extern crate malloc;

use interface::bdev::BlkReq;
use interface::bdev::NvmeBDev;
use interface::bdev::BSIZE;
use interface::error::{ErrorKind, Result};
use interface::rref::{RRef, RRefDeque};
use interface::{net::Net, rpc::RpcResult};

pub struct NullBlk {}

impl NullBlk {
    pub fn new() -> Self {
        Self {}
    }
}

impl interface::bdev::NvmeBDev for NullBlk {
    fn submit_and_poll_rref(
        &self,
        mut submit: RRefDeque<BlkReq, 128>,
        mut collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)>> {
        unimplemented!();
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<BlkReq, 1024>,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>> {
        unimplemented!();
    }

    fn get_stats(&self) -> RpcResult<Result<(u64, u64)>> {
        unimplemented!();
    }
}
