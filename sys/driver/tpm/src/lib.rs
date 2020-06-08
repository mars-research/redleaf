#![no_std]
#![feature(
    const_fn,
    const_raw_ptr_to_usize_cast,
    asm,
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

fn read_tpm_id(tpm: &Tpm, locality: u32) {
    let did_vid = tpm.read_u32(locality, TpmRegs::TPM_DID_VID);

    let did = (did_vid >> 16) & 0xFFFF;
    let vid = u16::from_be(did_vid as u16);
    println!("Locality {} => VID_DID: 0x{:x}", locality, vid);
}

fn tpm_validate_locality(tpm: &Tpm, locality: u32) -> bool {
    let timeout = 100;
    for i in (0..timeout).rev() {
        let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
        let mut reg_acc = TpmAccess(reg);
        if reg_acc.tpm_reg_validsts() && !reg_acc.seize() {
            return true;
        }
        unsafe { asm!("pause"); }
    }

    return false;
}

fn relinquish_locality(tpm: &Tpm, locality: u32) -> bool {
    let mut reg_acc = TpmAccess(0);
    reg_acc.set_active_locality(true);

    tpm.write_u8(locality, TpmRegs::TPM_ACCESS, reg_acc.bit_range(7, 0));

    for i in (0..tpm.timeout_a).rev() {
        let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
        let mut reg_acc = TpmAccess(reg);
        if reg_acc.tpm_reg_validsts() && !reg_acc.active_locality() {
            return true;
        }
    }

    return false;
}

fn request_locality(tpm: &Tpm, locality: u32) -> bool {
    let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
    let mut reg_acc = TpmAccess(reg);

    if !reg_acc.tpm_reg_validsts() {
        return false;
    }

    if reg_acc.active_locality() {
        return true;
    }

    let mut reg_acc = TpmAccess(0);
    reg_acc.set_request_use(true);

    tpm.write_u8(locality, TpmRegs::TPM_ACCESS, reg_acc.bit_range(7, 0));

    for i in (0..tpm.timeout_a).rev() {
        let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
        let mut reg_acc = TpmAccess(reg);
        if reg_acc.tpm_reg_validsts() && reg_acc.active_locality() {
            return true;
        }
    }

    return false;
}

fn tpm_get_burst(tpm: &Tpm) -> u16 {
    let reg_sts = tpm.read_u32(0, TpmRegs::TPM_STS);
    println!("{:x?}", u32::to_le_bytes(reg_sts));
    (reg_sts >> 8) as u16 & 0xFFFF
}

fn tpm_write_data(tpm: &Tpm, locality: u32, data: &[u8]) {
    let burst_count = tpm_get_burst(tpm) as usize;

    let mut data = data;

    while data.len() > burst_count {
        let (data0, data1) = data.split_at(burst_count);
        tpm.write_data(locality, TpmRegs::TPM_DATA_FIFO, data0); 
        data = data1;
    }

    // data is written to the FIFO
    let mut reg_sts = TpmStatus(0);
    reg_sts.set_tpm_go(true);

    // Execute the command using TPM.go
    tpm.write_u8(locality, TpmRegs::TPM_STS, reg_sts.bit_range(7, 0));
}

fn is_data_available(tpm: &Tpm, locality: u32) -> bool {
    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
    let status = libtpm::TpmStatus(reg_sts);

    for _ in (0..tpm.timeout_a).rev() {
        if status.sts_valid() && status.data_avail() {
            return true;
        }
    }
    return false;
}

fn tpm_read_data(tpm: &Tpm, locality: u32, data: &mut [u8]) {

    if is_data_available(tpm, locality) {
        let burst_count = tpm_get_burst(tpm) as usize;

        let mut data = data;

        while data.len() > burst_count {
            let (data0, data1) = data.split_at_mut(burst_count);
            tpm.read_data(locality, TpmRegs::TPM_DATA_FIFO, data0); 
            data = data1;
        }

        // data is written to the FIFO
        let mut reg_sts = TpmStatus(0);
        reg_sts.set_command_ready(true);

        // Execute the command using TPM.go
        tpm.write_u8(locality, TpmRegs::TPM_STS, reg_sts.bit_range(7, 0));
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

    println!("STS {}", reg_sts);

    println!("burst_count {}", tpm_get_burst(&tpm));
    println!("validate {}", tpm_validate_locality(&tpm, 0));

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
