#![no_std]
#![no_main]
#![feature(asm, box_syntax, const_fn, const_raw_ptr_to_usize_cast)]
#![forbid(unsafe_code)]

mod tpm_dev;
mod usr_tpm;

extern crate alloc;
extern crate b2histogram;
extern crate malloc;
#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate bitfield;

#[macro_use]
use alloc::boxed::Box;

#[macro_use]
use core::panic::PanicInfo;
use syscalls::{Heap, Syscall};

use console::println;
use libsyscalls::syscalls::sys_backtrace;
pub use interface::error::{ErrorKind, Result};

use interface::tpm::TpmRegs;

pub const ONE_MS_IN_NS: u64 = 1000 * 1000;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
) -> Box<dyn interface::tpm::UsrTpm> {
    libsyscalls::syscalls::init(s);

    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("tpm_init: =>  starting tpm driver domain");

    let tpm: Box<dyn interface::tpm::TpmDev> = box tpm_dev::Tpm::new();

    let rev_id = tpm.read_u8(0, TpmRegs::TPM_RID);
    println!("RID {:x?}", rev_id);

    let reg_acc = tpm.read_u8(0, TpmRegs::TPM_ACCESS);
    println!("ACCESS {:x?}", reg_acc);

    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
    let _status = libtpm::TpmStatus(reg_sts);

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
