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
                 heap: Box<dyn Heap + Send + Sync>) -> Box<dyn usr::tpm::TpmDev> {
    libsyscalls::syscalls::init(s);

    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("tpm_init: =>  starting tpm driver domain");

    let tpm: Box<dyn usr::tpm::TpmDev> = box tpm_dev::Tpm::new();

    println!("Starting tests");

    for i in 0..5 {
        read_tpm_id(&*tpm, i);
    }

    let rev_id = tpm.read_u8(0, TpmRegs::TPM_RID);
    println!("RID {:x?}", rev_id);

    let reg_acc = tpm.read_u8(0, TpmRegs::TPM_ACCESS);
    println!("ACCESS {:x?}", reg_acc);

    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
    let status = libtpm::TpmStatus(reg_sts);

    println!("STS {:x?}", reg_sts);

    // Changing locality
    let locality = 0;
    println!("burst_count {}", tpm_get_burst(&*tpm, locality));
    // Initially we have locality 0
    println!("request locality {}", tpm_request_locality(&*tpm, locality));
    println!("validate locality {}", tpm_validate_locality(&*tpm, locality));
    // Deactivate all localities
    tpm_deactivate_all_localities(&*tpm);
    let locality = 2;
    // Then request target localities
    println!("request locality {}", tpm_request_locality(&*tpm, locality));
    println!("validate locality {}", tpm_validate_locality(&*tpm, locality));

    // Get 1 byte of random value
    println!("random {}", tpm_get_random(&*tpm, locality, 1));

    // PCR extend
    // First we obtain "banks" that are allocated in the TPM.
    // In TPM2, there can be multiple banks, each implementing different hash algorithms.
    let tpm_info = tpm_get_pcr_allocation(&*tpm, locality);
    let mut digests: Vec<TpmTHa> = Vec::new();
    for i in 0..(tpm_info.nr_allocated_banks as usize) {
        let mut digest: Vec<u8> = Vec::new();
        digest.extend([0].repeat(tpm_info.allocated_banks[i].digest_size as usize));
        let tpm_digest = TpmTHa::new(tpm_info.allocated_banks[i].alg_id, digest);
        digests.push(tpm_digest);
    }
    let mut pcr_size: u16 = 0 as u16;
    let mut pcr: Vec<u8> = Vec::new();
    let pcr_idx = 17;
    // Read the initial value of the PCR that we want to extend
    tpm_pcr_read(&*tpm, locality, pcr_idx, TpmAlgorithms::TPM_ALG_SHA256 as u16, &mut pcr_size, &mut pcr);
    println!("pre-extend pcr {:x?}", pcr);
    println!("pcr_size {}", pcr_size);
    // Then extend the PCR
    tpm_pcr_extend(&*tpm, locality, &tpm_info, pcr_idx, digests);
    pcr_size = 0 as u16;
    pcr.clear();
    // Check the value of the PCR after extending
    tpm_pcr_read(&*tpm, locality, pcr_idx, TpmAlgorithms::TPM_ALG_SHA256 as u16, &mut pcr_size, &mut pcr);
    println!("post-extend pcr {:x?}", pcr);
    println!("pcr_size {}", pcr_size);

    // Sealing Data
    // Create Primary key (a.k.a. Storate Root Key)
    let primary_unique = b"data_sealing";
    let mut primary_pubkey_size: usize = 0;
    let mut primary_pubkey: Vec<u8> = Vec::new();
    let mut parent_handle: u32 = 0 as u32;
    tpm_create_primary(&*tpm, locality, None, primary_unique,
                       /*restricted=*/true, /*decrypt=*/true, /*sign=*/false,
                       &mut parent_handle, &mut primary_pubkey_size, &mut primary_pubkey);
    println!("parent_handle {:x?}", parent_handle);
    // Start authenticated session
    let mut session_handle: u32 = 0 as u32;
    let nonce = alloc::vec![0; 32];
    tpm_start_auth_session(&*tpm, locality, TpmSE::TPM_SE_TRIAL, nonce, &mut session_handle);
    // Tie session to PCR 17
    tpm_policy_pcr(&*tpm, locality, session_handle, b"".to_vec(), pcr_idx);
    // Get digest of authenticated session
    let mut policy_digest: Vec<u8> = Vec::new();
    tpm_policy_get_digest(&*tpm, locality, session_handle, &mut policy_digest);
    // Create Child key wrapped with SRK
    // Load Child key to TPM
    // Seal data under PCR 17 using Child key
    let mut create_out_private: Vec<u8> = Vec::new();
    let mut create_out_public: Vec<u8> = Vec::new();
    let sensitive_data: Vec<u8> = b"horizon".to_vec();
    tpm_create(&*tpm, locality, None, parent_handle, policy_digest, sensitive_data,
               /*restricted=*/false, /*decrypt=*/false, /*sign=*/false,
               &mut create_out_private, &mut create_out_public);
    let mut item_handle: u32 = 0 as u32;
    tpm_load(&*tpm, locality, parent_handle,
             create_out_private, create_out_public, &mut item_handle);

    // Unsealing Data
    // Start authenticated session
    let mut unseal_session_handle: u32 = 0 as u32;
    let nonce = alloc::vec![0; 32];
    tpm_start_auth_session(&*tpm, locality, TpmSE::TPM_SE_POLICY,
                           nonce, &mut unseal_session_handle);
    // Tie session to PCR 17
    tpm_policy_pcr(&*tpm, locality, unseal_session_handle, b"".to_vec(), pcr_idx);
    // Unseal data under PCR 17 using Child key (should succeed)
    let mut out_data: Vec<u8> = Vec::new();
    tpm_unseal(&*tpm, locality, unseal_session_handle, item_handle, &mut out_data);

    // Unload all objects from TPM memory
    tpm_flush_context(&*tpm, locality, parent_handle);
    tpm_flush_context(&*tpm, locality, session_handle);
    tpm_flush_context(&*tpm, locality, item_handle);
    tpm_flush_context(&*tpm, locality, unseal_session_handle);

    // Create Attestation Identity Key
    let     aik_unique = b"remote_attestation";
    let mut aik_pubkey_size: usize = 0;
    let mut aik_pubkey: Vec<u8> = Vec::new();
    let mut aik_handle: u32 = 0 as u32;
    tpm_create_primary(&*tpm, locality, None, aik_unique,
                       /*restricted=*/true, /*decrypt=*/false, /*sign=*/true,
                       &mut aik_handle, &mut aik_pubkey_size, &mut aik_pubkey);
    println!("aik_handle {:x?}", aik_handle);
    // Generate random nonce. This should be generated by remote verifier.
    let nonce = b"random_nonce";
    // Prepare vector of indexes of PCRs
    let mut pcr_idxs: Vec<usize> = Vec::new();
    pcr_idxs.push(0);
    // Request quote
    let mut out_pcr_digest: Vec<u8> = Vec::new();
    let mut out_sig: Vec<u8> = Vec::new();
    tpm_quote(&*tpm, locality, aik_handle, TpmAlgorithms::TPM_ALG_SHA256 as u16,
              nonce.to_vec(), pcr_idxs,
              &mut out_pcr_digest, &mut out_sig);
    println!("out_pcr_digest {:x?}", out_pcr_digest);
    println!("out_sig {:x?}", out_sig);

    tpm
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}