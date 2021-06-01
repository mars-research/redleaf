#![no_std]
#![feature(slice_fill)] // for vec::fill
#![feature(core_intrinsics)] // prefetch helpers

extern crate alloc;
extern crate core;

use alloc::vec::Vec;
use console::println;
use interface::tpm::*;

pub fn test_tpm(tpm: &dyn UsrTpm) {
    println!("Starting tests");

    for i in 0..5 {
        tpm.read_tpm_id(i);
    }

    // Changing locality
    let locality = 0;
    println!("burst_count {}", tpm.tpm_get_burst(locality).unwrap());
    // Initially we have locality 0
    println!("request locality {}", tpm.tpm_request_locality(locality).unwrap());
    println!("validate locality {}", tpm.tpm_validate_locality(locality).unwrap());
    // Deactivate all localities
    tpm.tpm_deactivate_all_localities();
    let locality = 2;
    // Then request target localities
    println!("request locality {}", tpm.tpm_request_locality(locality).unwrap());
    println!("validate locality {}", tpm.tpm_validate_locality(locality).unwrap());

    // Get 1 byte of random value
    println!("random {}", tpm.tpm_get_random(locality, 1).unwrap());

    // PCR extend
    // First we obtain "banks" that are allocated in the TPM.
    // In TPM2, there can be multiple banks, each implementing different hash algorithms.
    let tpm_info = tpm.tpm_get_pcr_allocation(locality).unwrap();
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
    tpm.tpm_pcr_read(
        locality,
        pcr_idx,
        TpmAlgorithms::TPM_ALG_SHA256 as u16,
        &mut pcr_size,
        &mut pcr,
    ).unwrap();
    println!("pre-extend pcr {:x?}", pcr);
    println!("pcr_size {}", pcr_size);
    // Then extend the PCR
    tpm.tpm_pcr_extend(locality, &tpm_info, pcr_idx, digests);
    pcr_size = 0 as u16;
    pcr.clear();
    // Check the value of the PCR after extending
    tpm.tpm_pcr_read(
        locality,
        pcr_idx,
        TpmAlgorithms::TPM_ALG_SHA256 as u16,
        &mut pcr_size,
        &mut pcr,
    ).unwrap();
    println!("post-extend pcr {:x?}", pcr);
    println!("pcr_size {}", pcr_size);

    // Sealing Data
    // Create Primary key (a.k.a. Storate Root Key)
    let primary_unique = b"data_sealing";
    let mut primary_pubkey_size: usize = 0;
    let mut primary_pubkey: Vec<u8> = Vec::new();
    let mut parent_handle: u32 = 0 as u32;
    tpm.tpm_create_primary(
        locality,
        None,
        primary_unique,
        /*restricted=*/ true,
        /*decrypt=*/ true,
        /*sign=*/ false,
        &mut parent_handle,
        &mut primary_pubkey_size,
        &mut primary_pubkey,
    ).unwrap();
    println!("parent_handle {:x?}", parent_handle);
    // Start authenticated session
    let mut session_handle: u32 = 0 as u32;
    let nonce = alloc::vec![0; 32];
    tpm.tpm_start_auth_session(locality, TpmSE::TPM_SE_TRIAL, nonce, &mut session_handle).unwrap();
    // Tie session to PCR 17
    tpm.tpm_policy_pcr(locality, session_handle, b"".to_vec(), pcr_idx).unwrap();
    // Get digest of authenticated session
    let mut policy_digest: Vec<u8> = Vec::new();
    tpm.tpm_policy_get_digest(locality, session_handle, &mut policy_digest).unwrap();
    // Create Child key wrapped with SRK
    // Load Child key to TPM
    // Seal data under PCR 17 using Child key
    let mut create_out_private: Vec<u8> = Vec::new();
    let mut create_out_public: Vec<u8> = Vec::new();
    let sensitive_data: Vec<u8> = b"horizon".to_vec();
    tpm.tpm_create(
        locality,
        None,
        parent_handle,
        policy_digest,
        sensitive_data,
        /*restricted=*/ false,
        /*decrypt=*/ false,
        /*sign=*/ false,
        &mut create_out_private,
        &mut create_out_public,
    ).unwrap();
    let mut item_handle: u32 = 0 as u32;
    tpm.tpm_load(
        locality,
        parent_handle,
        create_out_private,
        create_out_public,
        &mut item_handle,
    ).unwrap();

    // Unsealing Data
    // Start authenticated session
    let mut unseal_session_handle: u32 = 0 as u32;
    let nonce = alloc::vec![0; 32];
    tpm.tpm_start_auth_session(
        locality,
        TpmSE::TPM_SE_POLICY,
        nonce,
        &mut unseal_session_handle,
    ).unwrap();
    // Tie session to PCR 17
    tpm.tpm_policy_pcr(locality, unseal_session_handle, b"".to_vec(), pcr_idx).unwrap();
    // Unseal data under PCR 17 using Child key (should succeed)
    let mut out_data: Vec<u8> = Vec::new();
    tpm.tpm_unseal(locality, unseal_session_handle, item_handle, &mut out_data).unwrap();

    // Unload all objects from TPM memory
    tpm.tpm_flush_context(locality, parent_handle).unwrap();
    tpm.tpm_flush_context(locality, session_handle).unwrap();
    tpm.tpm_flush_context(locality, item_handle).unwrap();
    tpm.tpm_flush_context(locality, unseal_session_handle).unwrap();

    // Create Attestation Identity Key
    let aik_unique = b"remote_attestation";
    let mut aik_pubkey_size: usize = 0;
    let mut aik_pubkey: Vec<u8> = Vec::new();
    let mut aik_handle: u32 = 0 as u32;
    tpm.tpm_create_primary(
        locality,
        None,
        aik_unique,
        /*restricted=*/ true,
        /*decrypt=*/ false,
        /*sign=*/ true,
        &mut aik_handle,
        &mut aik_pubkey_size,
        &mut aik_pubkey,
    ).unwrap();
    println!("aik_handle {:x?}", aik_handle);
    // Generate random nonce. This should be generated by remote verifier.
    let nonce = b"random_nonce";
    // Prepare vector of indexes of PCRs
    let mut pcr_idxs: Vec<usize> = Vec::new();
    pcr_idxs.push(0);
    // Request quote
    let mut out_pcr_digest: Vec<u8> = Vec::new();
    let mut out_sig: Vec<u8> = Vec::new();
    tpm.tpm_quote(
        locality,
        aik_handle,
        TpmAlgorithms::TPM_ALG_SHA256 as u16,
        nonce.to_vec(),
        pcr_idxs,
        &mut out_pcr_digest,
        &mut out_sig,
    ).unwrap();
    println!("out_pcr_digest {:x?}", out_pcr_digest);
    println!("out_sig {:x?}", out_sig);
}
