#![no_std]
#![no_main]
#![feature(
    asm,
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
)]
#![forbid(unsafe_code)]

mod tpm_dev;
mod usr_tpm;

extern crate malloc;
extern crate alloc;
extern crate b2histogram;
#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate bitfield;

use libtpm::*;
use bitfield::BitRange;

#[macro_use]
use b2histogram::Base2Histogram;
use byteorder::{ByteOrder, BigEndian};

use libtime::sys_ns_loopsleep;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
#[macro_use]
use alloc::vec::Vec;
use alloc::vec;
use core::panic::PanicInfo;
use syscalls::{Syscall, Heap};
use usr;
use usr::rpc::RpcResult;
use console::{println, print};
use libsyscalls::syscalls::sys_backtrace;
pub use usr::error::{ErrorKind, Result};
use core::cell::RefCell;
use core::{mem, ptr};
use tpm_device::TpmDevice; 
use usr::tpm::TpmRegs;
use libtime::get_rdtsc as rdtsc;
use libtpm::*;

pub const ONE_MS_IN_NS: u64 = 1000 * 1000;

#[no_mangle]
pub fn trusted_entry(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::tpm::UsrTpm> {
    libsyscalls::syscalls::init(s);

    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("tpm_init: =>  starting tpm driver domain");

    let tpm: Box<dyn usr::tpm::TpmDev> = box tpm_dev::Tpm::new();

    let rev_id = tpm.read_u8(0, TpmRegs::TPM_RID);
    println!("RID {:x?}", rev_id);

    let reg_acc = tpm.read_u8(0, TpmRegs::TPM_ACCESS);
    println!("ACCESS {:x?}", reg_acc);

    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
    let status = libtpm::TpmStatus(reg_sts);

    println!("STS {:x?}", reg_sts);

    let usrtpm = box usr_tpm::UsrTpm::new(tpm);

    #[cfg(feature = "testtpm")]
    libbenchtpm::test_tpm(&*usrtpm);

    usrtpm
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}