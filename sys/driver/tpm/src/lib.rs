#![no_std]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions,
        maybe_uninit_extra
)]
#![forbid(unsafe_code)]

mod libtpm;

extern crate malloc;
extern crate alloc;
extern crate b2histogram;
#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate bitfield;

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

struct Tpm {
    device: TpmDevice,
    device_initialized: bool,
}

impl Tpm {
    fn new() -> Self{
        Self {
            device: TpmDevice::new(),
            device_initialized: true,
        }
    }

    fn active(&self) -> bool {
        self.device_initialized
    }

    fn read_reg(&self, locality: u32, reg: TpmRegs, buf: &mut Vec<u8>) {
        self.device.read_reg(locality, reg, buf);
    }

    fn write_reg(&self, locality: u32, reg: TpmRegs, buf: &Vec<u8>) {
        self.device.write_reg(locality, reg, buf);
    }

}

impl usr::tpm::TpmDev for Tpm {
    fn read_reg(&self, locality: u32, reg: TpmRegs, buf: &mut Vec<u8>) {
        self.device.read_reg(locality, reg, buf);
    }

    fn write_reg(&self, locality: u32, reg: TpmRegs, buf: &Vec<u8>) {
        self.device.write_reg(locality, reg, buf);
    }
}

fn read_tpm_id(tpm: &Tpm, locality: u32) {
    let mut v = alloc::vec![0u8; 4];

    tpm.read_reg(locality, TpmRegs::TPM_DID_VID, &mut v);

    println!("Locality {} => VID_DID: {:x?}", locality, v);
}

#[no_mangle]
pub fn tpm_init(s: Box<dyn Syscall + Send + Sync>,
                 heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::tpm::TpmDev> {
    libsyscalls::syscalls::init(s);

    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("tpm_init: =>  starting tpm driver domain");

    let tpm = Tpm::new();

    println!("Starting tests");

    for i in 0..5 {
        read_tpm_id(&tpm, i);
    }

    let mut v = alloc::vec![0u8];

    tpm.read_reg(0, TpmRegs::TPM_RID, &mut v);

    println!("RID {:x?}", v);

    let mut v = alloc::vec![0u8];
    tpm.read_reg(0, TpmRegs::TPM_ACCESS, &mut v);
    println!("ACCESS {:x?}", v);

    let mut v = alloc::vec![0u8; 3];
    tpm.read_reg(0, TpmRegs::TPM_STS, &mut v);
    let m = libtpm::TpmStatus(v);

    println!("STS burst {:x} {}", m.burst_count(), m.data_avail());

    panic!("");

    Box::new(tpm)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
