#![no_std]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    asm,
)]
#![forbid(unsafe_code)]

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

struct Tpm {
    device: TpmDevice,
    device_initialized: bool,
    timeout_a: usize,
}

impl Tpm {
    fn new() -> Self{
        Self {
            device: TpmDevice::new(),
            device_initialized: true,
            timeout_a: 750,
        }
    }

    #[inline(always)]
    fn active(&self) -> bool {
        self.device_initialized
    }

    #[inline(always)]
    fn read_u8(&self, locality: u32, reg: TpmRegs) -> u8 {
        self.device.read_u8(locality, reg)
    }

    #[inline(always)]
    fn write_u8(&self, locality: u32, reg: TpmRegs, val: u8) {
        self.device.write_u8(locality, reg, val);
    }

    #[inline(always)]
    fn read_u32(&self, locality: u32, reg: TpmRegs) -> u32 {
        self.device.read_u32(locality, reg)
    }

    #[inline(always)]
    fn write_u32(&self, locality: u32, reg: TpmRegs, val: u32) {
        self.device.write_u32(locality, reg, val);
    }

    #[inline(always)]
    fn read_data(&self, locality: u32, reg: TpmRegs, buf: &mut [u8]) {
        for byte in buf.iter_mut() {
            *byte = self.read_u8(locality, reg);
        }
    }

    #[inline(always)]
    fn write_data(&self, locality: u32, reg: TpmRegs, buf: &[u8]) {
        for byte in buf.iter() {
            self.write_u8(locality, reg, *byte);
        }
    }
}

impl usr::tpm::TpmDev for Tpm {
    fn read_u8(&self, locality: u32, reg: TpmRegs) -> u8 {
        self.device.read_u8(locality, reg)
    }

    fn write_u8(&self, locality: u32, reg: TpmRegs, val: u8) {
        self.device.write_u8(locality, reg, val);
    }

    fn read_u32(&self, locality: u32, reg: TpmRegs) -> u32 {
        self.device.read_u32(locality, reg)
    }

    fn write_u32(&self, locality: u32, reg: TpmRegs, val: u32) {
        self.device.write_u32(locality, reg, val);
    }
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

    let rev_id = tpm.read_u8(0, TpmRegs::TPM_RID);
    println!("RID {:x?}", rev_id);

    let reg_acc = tpm.read_u8(0, TpmRegs::TPM_ACCESS);
    println!("ACCESS {:x?}", reg_acc);

    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
    let status = libtpm::TpmStatus(reg_sts);

    println!("STS {:x?}", reg_sts);

    let mut locality = 0;
    println!("burst_count {}", tpm_get_burst(&tpm, locality));
    // Initially we have locality 0
    println!("request locality {}", tpm_request_locality(&tpm, locality));
    println!("validate locality {}", tpm_validate_locality(&tpm, locality));
    // Deactivate all localities
    tpm_deactivate_all_localities(&tpm);
    let mut locality = 2;
    // Then request target localities
    println!("request locality {}", tpm_request_locality(&tpm, locality));
    println!("validate locality {}", tpm_validate_locality(&tpm, locality));

    println!("random {}", tpm_get_random(&tpm, locality, 1));
    let tpm_info = tpm_get_pcr_allocation(&tpm, locality);
    let mut digests: Vec<TpmDigest> = Vec::new();
    for i in 0..(tpm_info.nr_allocated_banks as usize) {
        let mut digest: Vec<u8> = Vec::new();
        digest.extend([0].repeat(tpm_info.allocated_banks[i].digest_size as usize));
        let tpm_digest = TpmDigest::new(tpm_info.allocated_banks[i].alg_id, digest);
        digests.push(tpm_digest);
    }
    let mut pcr_size: u16 = 0 as u16;
    let mut pcr: Vec<u8> = Vec::new();
    let pcr_idx = 17;
    tpm_pcr_read(&tpm, locality, pcr_idx, TpmAlgorithms::TPM_ALG_SHA256 as u16, &mut pcr_size, &mut pcr);
    println!("pre-extend pcr {:x?}", pcr);
    println!("pcr_size {}", pcr_size);
    tpm_pcr_extend(&tpm, locality, &tpm_info, pcr_idx, digests);
    pcr_size = 0 as u16;
    pcr.clear();
    tpm_pcr_read(&tpm, locality, pcr_idx, TpmAlgorithms::TPM_ALG_SHA256 as u16, &mut pcr_size, &mut pcr);
    println!("post-extend pcr {:x?}", pcr);
    println!("pcr_size {}", pcr_size);

    Box::new(tpm)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
