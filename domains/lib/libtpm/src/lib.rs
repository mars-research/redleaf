/*
This library implements TPM Commands from the Trusted Computing Group
TPM 2.0 Library specification.
TPM Commands are APIs exposed to the user application so that the
application can utilize functions implemented in the TPM, such as
crypto operations.
Please refer to the individual specification shown in the specification
for more detail.
This library implements the specification proposed in the following link:
https://trustedcomputinggroup.org/wp-content/uploads/TCG_TPM2_r1p59_Part3_Commands_pub.pdf
*/

#![no_std]

extern crate alloc;
extern crate malloc;
extern crate byteorder;

#[macro_use]
extern crate bitfield;

mod regs;
mod datastructure;

use alloc::vec::Vec;
use bitfield::BitRange;
use console::{print, println};
use libtime::sys_ns_loopsleep;
use usr::tpm::{TpmDev, TpmRegs};
pub use regs::*;
pub use datastructure::*;
use byteorder::{ByteOrder, BigEndian};
use sha2::{Digest, Sha256};

pub const ONE_MS_IN_NS: u64 = 1000 * 1000;

macro_rules! tpm_send_command_ready {
    ($t: ident, $l: ident) => {
        let mut reg_sts = TpmStatus(0);
        reg_sts.set_command_ready(true);

        $t.write_u8($l, TpmRegs::TPM_STS, reg_sts.bit_range(7, 0));
    }
}

#[inline(always)]
pub fn read_data(tpm: &dyn TpmDev, locality: u32, reg: TpmRegs, buf: &mut [u8]) {
    for byte in buf.iter_mut() {
        *byte = tpm.read_u8(locality, reg);
    }
}

#[inline(always)]
pub fn write_data(tpm: &dyn TpmDev, locality: u32, reg: TpmRegs, buf: &[u8]) {
    for byte in buf.iter() {
        tpm.write_u8(locality, reg, *byte);
    }
}

pub fn tpm_buf_append_u16(buf: &mut Vec<u8>, data: u16) {
    buf.extend_from_slice(&u16::to_be_bytes(data));
}

pub fn tpm_buf_append_u32(buf: &mut Vec<u8>, data: u32) {
    buf.extend_from_slice(&u32::to_be_bytes(data));
}

pub fn tpm_buf_append(buf: &mut Vec<u8>, data: &Vec <u8>) {
    buf.extend(data);
}
/// ## Locality related functions
///
/// Locality tells the TPM where the command originated.
/// Validates the TPM locality, basically means that TPM is ready to listen for commands and
/// perform operation in this locality.
/// Ref: https://ebrary.net/24811/computer_science/locality_command
pub fn tpm_validate_locality(tpm: &dyn TpmDev, locality: u32) -> bool {
    let timeout = 100;
    for i in (0..timeout).rev() {
        let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
        let mut reg_acc = TpmAccess(reg);
        if reg_acc.tpm_reg_validsts() && !reg_acc.seize() {
            println!("Validate locality access {:x?}", reg_acc);
            return true;
        }
    }

    return false;
}

/// Explicitly giveup locality. This may not be useful if there is only a single process/user using
/// TPM in an OS. In multi-user scenario, this is more applicable.
pub fn relinquish_locality(tpm: &dyn TpmDev, locality: u32) -> bool {
    let mut reg_acc = TpmAccess(0);
    reg_acc.set_active_locality(true);

    tpm.write_u8(locality, TpmRegs::TPM_ACCESS, reg_acc.bit_range(7, 0));

    for i in (0..TIMEOUT_A).rev() {
        let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
        let mut reg_acc = TpmAccess(reg);
        if reg_acc.tpm_reg_validsts() && !reg_acc.active_locality() {
            return true;
        }
        sys_ns_loopsleep(ONE_MS_IN_NS);
    }

    return false;
}

pub fn tpm_deactivate_all_localities(tpm: &dyn TpmDev) -> bool {
    let mut reg_acc = TpmAccess(0);
    reg_acc.set_active_locality(true);
    for locality in 0..3 {
        tpm.write_u8(locality, TpmRegs::TPM_ACCESS, reg_acc.bit_range(7, 0));
    }
    return true;
}

/// Requests the TPM to switch to the locality we choose and wait for TPM to acknowledge our
/// request
pub fn tpm_request_locality(tpm: &dyn TpmDev, locality: u32) -> bool {
    let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
    let mut reg_acc = TpmAccess(reg);
    println!("Request locality access {:x?}", reg_acc);

    if !reg_acc.tpm_reg_validsts() {
        return false;
    }

    if reg_acc.active_locality() {
        return true;
    }

    let mut reg_acc = TpmAccess(0);
    reg_acc.set_request_use(true);
    reg_acc.set_active_locality(true);

    tpm.write_u8(locality, TpmRegs::TPM_ACCESS, reg_acc.bit_range(7, 0));

    for i in (0..TIMEOUT_A).rev() {
        let reg = tpm.read_u8(locality, TpmRegs::TPM_ACCESS);
        let mut reg_acc = TpmAccess(reg);
        if reg_acc.tpm_reg_validsts() && reg_acc.active_locality() {
            return true;
        }
        sys_ns_loopsleep(ONE_MS_IN_NS);
    }

    return false;
}

/// Reads the TPM ID from device register
pub fn read_tpm_id(tpm: &dyn TpmDev, locality: u32) {
    let did_vid = tpm.read_u32(locality, TpmRegs::TPM_DID_VID);

    let did = (did_vid >> 16) & 0xFFFF;
    let vid = u16::from_be(did_vid as u16);
    println!("Locality {} => VID_DID: 0x{:x}", locality, vid);
}

/// Reads the burst_count from TPM register. Burst count is the amount of bytes the TPM device is
/// capable of handling in oneshot.
pub fn tpm_get_burst(tpm: &TpmDev, locality: u32) -> u16 {
    let reg_sts = tpm.read_u32(locality, TpmRegs::TPM_STS);
    println!("{:x?}", u32::to_le_bytes(reg_sts));
    (reg_sts >> 8) as u16 & 0xFFFF
}

/// Busy-wait in a loop for a particular status flag to be set
pub fn wait_for_status_flag(tpm: &TpmDev, locality: u32, flag: u8, timeout_ms: usize) -> bool {

    for _ in 0..timeout_ms {
        let mut reg_sts = tpm.read_u8(locality, TpmRegs::TPM_STS);
        let mut status: TpmStatus = TpmStatus(reg_sts);

        if reg_sts & flag == flag {
            return true;
        }
        sys_ns_loopsleep(ONE_MS_IN_NS);
    }
    return false;
}

/// Writes data to the TPM FIFO.
/// Here, `data.len < burst_count`
pub fn tpm_write_data(tpm: &dyn TpmDev, locality: u32, data: &[u8]) -> usize {
    let burst_count = tpm_get_burst(tpm, locality) as usize;

    if data.len() > burst_count {
        println!("Data size > burst_count! not supported yet");
        return 0;
    }

    for byte in data.iter() {
        tpm.write_u8(locality, TpmRegs::TPM_DATA_FIFO, *byte); 
    }

    // data is written to the FIFO
    let mut reg_sts = TpmStatus(0);
    reg_sts.set_tpm_go(true);

    // Execute the command using TPM.go
    tpm.write_u8(locality, TpmRegs::TPM_STS, reg_sts.bit_range(7, 0));

    return data.len();
}

/// Checks TPM status register to see if there is any data available
pub fn is_data_available(tpm: &dyn TpmDev, locality: u32) -> bool {
    let reg_sts = tpm.read_u8(locality, TpmRegs::TPM_STS);
    let status = TpmStatus(reg_sts);

    for _ in (0..TIMEOUT_A).rev() {
        if status.sts_valid() && status.data_avail() {
            return true;
        }
    }
    return false;
}

/// Read data from TPM
/// * Wait for data to be available
/// * Receive as much as burst_count
pub fn tpm_read_data(tpm: &dyn TpmDev, locality: u32, data: &mut [u8]) -> usize {
    let reg_sts = tpm.read_u8(locality, TpmRegs::TPM_STS);
    let mut status = TpmStatus(reg_sts);

    status.set_sts_valid(true);
    status.set_data_avail(true);

    println!("Read before STS {:x?}", status);
    if !wait_for_status_flag(tpm, locality, status.bit_range(7, 0), TIMEOUT_C) {
        println!("tpm_read_data timeout");
        return 0;
    }
    println!("Read after STS {:x?}", status);

    let mut data = data;
    let mut size = 0;
    let mut burst_count = 0;

    loop {
        burst_count = tpm_get_burst(tpm, locality) as usize;

        if burst_count > (data.len() - size) {
            burst_count = data.len() - size;
        }
        for i in (0..burst_count) {
            data[i + size] = tpm.read_u8(locality, TpmRegs::TPM_DATA_FIFO);
        }
        size = size + burst_count;
        if size >= data.len() {
            break;
        }
    }

    return data.len();
}

/// Wrapper for `tpm_read_data`
/// This function first tries to read TPM_HEADER_SIZE bytes from the TPM to determine the length of
/// payload data.
/// Then it issues a second read for the length of payload data subtract TPM_HEADER_SIZE
/// Payload consists of the argument that was sent to the TPM during tpm_send_data and the response
pub fn tpm_recv_data(tpm: &TpmDev, locality: u32, buf: &mut Vec<u8>, rc: &mut u32) -> usize {
    let size = buf.len();
    let mut ret = 0;

    buf.clear();
    buf.extend([0].repeat(TPM_HEADER_SIZE));

    ret = tpm_read_data(tpm, locality, buf.as_mut_slice());
    if ret == 0 {
        println!("TPM did not return anything");
        buf.clear();
        *rc = Tpm2ReturnCodes::TPM2_RC_FAILURE as u32;
        return 0;
    }

    let hdr = TpmHeader::from_vec(buf);

    // Check whether TPM Return Code is TPM_SUCCESS
    if hdr.ordinal == (Tpm2ReturnCodes::TPM2_RC_RETRY as u32) {
        println!("TPM returned retry");
        *rc = hdr.ordinal;
        buf.clear();
        return 0;
    }
    else if hdr.ordinal != (Tpm2ReturnCodes::TPM2_RC_SUCCESS as u32) {
        println!("TPM returned with error {:x?}", hdr.ordinal);
        *rc = hdr.ordinal;
        buf.clear();
        return 0;
    }
    else {
        buf.clear();
        buf.extend([0].repeat(hdr.length as usize - TPM_HEADER_SIZE));
        *rc = hdr.ordinal;

        ret = tpm_read_data(tpm, locality, buf.as_mut_slice());

        tpm_send_command_ready!(tpm, locality);
    }

    return ret;
}

/// Wrapper for `tpm_write_data`
/// This function waits for TPM to be in a state to accept commands before writing data to FIFO.
pub fn tpm_send_data(tpm: &TpmDev, locality: u32, buf: &mut Vec<u8>) -> usize {
    let mut reg_sts = tpm.read_u8(locality, TpmRegs::TPM_STS);
    let mut status = TpmStatus(reg_sts);

    println!("Send before STS {:x?}", status);
    if !status.command_ready() {
        // If TPM is not ready, make it ready
        status.set_command_ready(true);
        tpm.write_u8(locality, TpmRegs::TPM_STS, status.bit_range(7, 0));

        if !wait_for_status_flag(tpm, locality, status.bit_range(7, 0), TIMEOUT_B) {
            println!("tpm_send_data timeout");
            return 0;
        }
    }
    println!("Send after STS {:x?}", status);

    return tpm_write_data(tpm, locality, buf.as_slice());
}

/// Transmit command to a TPM.
/// This function does a bi-directional communication with TPM.
/// First, it sends a command with headers
/// If successful, try to read the response buffer from TPM
pub fn tpm_transmit_cmd(tpm: &TpmDev, locality: u32, buf: &mut Vec<u8>) {
    let hdr = TpmHeader::from_vec(&buf);
    let mut rc: u32 = Tpm2ReturnCodes::TPM2_RC_NOT_USED as u32;
    let mut delay_msec: u64 = ONE_MS_IN_NS;
    let mut command_buf: Vec<u8> = buf.clone();

    println!("tpm_transmit_cmd len {} ord {:x}", hdr.length, hdr.ordinal);

    while rc != Tpm2ReturnCodes::TPM2_RC_SUCCESS as u32 {
        let tx_bytes = tpm_send_data(tpm, locality, &mut command_buf);

        println!("Transmitted {} bytes", tx_bytes);

        if tx_bytes > 0 {
            let rx_bytes = tpm_recv_data(tpm, locality, buf, &mut rc);

            println!("Received {} bytes", rx_bytes);
            if !(rc == Tpm2ReturnCodes::TPM2_RC_SUCCESS as u32 ||
                 rc == Tpm2ReturnCodes::TPM2_RC_RETRY as u32) {
                println!("TPM failed to respond to command");
                break;
            }
        }

        if delay_msec > (DURATION_LONG as u64) * ONE_MS_IN_NS {
            if rc == Tpm2ReturnCodes::TPM2_RC_RETRY as u32 {
                println!("TPM in retry loop");
            }
            break;
        }
        sys_ns_loopsleep(delay_msec);
        delay_msec = delay_msec * 2;
    }
}

/// Table 3:68 - TPM2_GetRandom Command
/// Get a random number from TPM. 
/// `num_octets` represents the length of the random number in bytes
pub fn tpm_get_random(tpm: &TpmDev, locality: u32, num_octets: usize) -> bool {
    let mut buf: Vec<u8>;

    // header: TPM_HEADER
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_GET_RANDOM as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // bytesRequested: u16
    buf.extend_from_slice(&(num_octets as u16).to_be_bytes());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);
    println!("postsend: {:x?}", buf);
    true
}

/// Table 3:114 - TPM2_PCR_Read Command
/// Read a PCR register.
/// Since the communication channel between the process and the TPM is untrusted,
/// TPM2_Quote should be the command to retreive PCR values, not TPM2_PCR_Read
pub fn tpm_pcr_read(tpm: &TpmDev, locality: u32, pcr_idx: usize, hash: u16,
                    digest_size: &mut u16, digest: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header: TPM_HEADER
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_PCR_READ as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // pcrSelection: TPMS_PCR_SELECTION
    let mut pcr_select: Vec<u8>;
    pcr_select = Vec::with_capacity(TPM_PCR_SELECT_MIN);
    pcr_select.extend([0].repeat(TPM_PCR_SELECT_MIN));
    pcr_select[pcr_idx >> 3] = 1 << (pcr_idx & 0x7);
    let pcr_selection = TpmSPcrSelection::new(
        hash,
        TPM_PCR_SELECT_MIN as u8,
        pcr_select
    );

    // pcrSelections: TPMS_PCR_SELECTION[]
    let count: u32 = 1;
    let mut pcr_selections: Vec<TpmSPcrSelection> = Vec::with_capacity(count as usize);
    pcr_selections.push(pcr_selection);

    // pcrSelectionIn: TPML_PCR_SELECTION
    let pcr_selection_in = TpmLPcrSelection::new(
        count,
        pcr_selections
    );
    buf.extend_from_slice(&pcr_selection_in.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        *digest_size = BigEndian::read_u16(&slice[18..20]);
        digest.extend([0].repeat(*digest_size as usize));
        digest.copy_from_slice(&slice[20..(20 + *digest_size as usize)]);
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Obtain information about banks that are allocated in TPM
pub fn tpm_init_bank_info(tpm: &TpmDev, locality: u32, hash_alg: u16) -> TpmBankInfo {
    let (mut crypto_id, mut digest_size) = match hash_alg {
        // Determine crypto_id and digest_size from hash_alg without calling tpm2_pcr_read
        hash_alg if hash_alg == TpmAlgorithms::TPM_ALG_SHA1 as u16 => 
            (HashAlgorithms::HASH_ALGO_SHA1 as u16, 20 as u16),
        hash_alg if hash_alg == TpmAlgorithms::TPM_ALG_SHA256 as u16 => 
            (HashAlgorithms::HASH_ALGO_SHA256 as u16, 32 as u16),
        hash_alg if hash_alg == TpmAlgorithms::TPM_ALG_SHA384 as u16 => 
            (HashAlgorithms::HASH_ALGO_SHA384 as u16, 48 as u16),
        hash_alg if hash_alg == TpmAlgorithms::TPM_ALG_SHA512 as u16 => 
            (HashAlgorithms::HASH_ALGO_SHA512 as u16, 64 as u16),
        hash_alg if hash_alg == TpmAlgorithms::TPM_ALG_SM3_256 as u16 => 
            (HashAlgorithms::HASH_ALGO_SM3_256 as u16, 32 as u16),
        // Determine crypto_id and digest_size from hash_alg by calling tpm2_pcr_read
        _ => {
            let mut size: u16 = 0;
            let mut digest: Vec<u8> = Vec::new();
            tpm_pcr_read(tpm, locality, 0, hash_alg as u16, &mut size, &mut digest);
            (HashAlgorithms::HASH_ALGO__LAST as u16, size)
        },
    };
    TpmBankInfo::new(hash_alg as u16, digest_size, crypto_id)
}

/// Table 3:208 - TPM2_PCR_GetCapability Command
/// Obtain the banks that are allocated in TPM
/// TODO: Return true/false, not structure
pub fn tpm_get_pcr_allocation(tpm: &TpmDev, locality: u32) -> TpmDevInfo {
    let mut buf: Vec<u8>;

    // header: TPM_HEADER
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_GET_CAPABILITY as u32
    );
    buf = TpmHeader::to_vec(&hdr);
    
    // capability: TPM_CAP
    buf.extend_from_slice(&(Tpm2Capabilities::TPM2_CAP_PCRS as u32).to_be_bytes());

    // property: u32
    buf.extend_from_slice(&(0 as u32).to_be_bytes());

    // propertyCount: u32
    buf.extend_from_slice(&(1 as u32).to_be_bytes());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    let mut nr_possible_banks: u32 = 0 as u32;
    let mut allocated_banks: Vec<TpmBankInfo> = Vec::new();
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        nr_possible_banks = BigEndian::read_u32(&slice[5..9]);
        println!("nr_possible_banks: {}", nr_possible_banks);
        let mut marker = 9;
        for i in 0..nr_possible_banks {
            let hash_alg: u16 = BigEndian::read_u16(&slice[marker..(marker + 2)]);
            let mut tpm_bank = tpm_init_bank_info(tpm, locality, hash_alg);
            println!("hash_alg: {:x?}, digest_size: {:x?}, crypto_id: {:x?}", tpm_bank.alg_id, tpm_bank.digest_size, tpm_bank.crypto_id);
            allocated_banks.push(tpm_bank);
            marker = marker + 6;
        }
    } else {
        println!("Didn't receive any response from TPM!");
    }
    TpmDevInfo::new(nr_possible_banks, allocated_banks)
}

/// Table 3:110 - TPM2_PCR_Read Command
/// Extend PCR register.
/// The value sent to the TPM will be concatenated with the original value and hashed.
pub fn tpm_pcr_extend(tpm: &TpmDev, locality: u32, tpm_info: &TpmDevInfo,
                      pcr_idx: usize, digest_values: Vec<TpmTHa>) -> bool {
    let mut buf: Vec<u8>;

    // header: TPM_HEADER
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_PCR_EXTEND as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // pcrHandle: TPMI_DH_PCR
    let handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16,
                                0 as u8, 0 as u16);
    let pcr_handle = TpmIDhPcr::new(pcr_idx as u32, handle);
    buf.extend_from_slice(&pcr_handle.to_vec());

    // digests: TPML_DIGEST_VALUES
    let digests = TpmLDigestValues::new(tpm_info.nr_allocated_banks, digest_values);
    buf.extend_from_slice(&digests.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Print resopnse
    println!("postsend: {:x?}", buf);
    true
}

/// Table 3:78 - TPM2_HashSequenceStart Command
/// Conduct hash calculation in TPM
pub fn tpm_hash_sequence_start(tpm: &TpmDev, locality: u32, hash: TpmAlgorithms, 
                               object: &mut u32) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_HASH_SEQUENCE_START as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // auth: Tpm2BAuth
    let auth = Tpm2BAuth::new(Vec::<u8>::new());
    buf.extend_from_slice(&auth.to_vec()); // auth.size

    // hashAlg: TpmIAlgHash
    let hash_alg = TpmIAlgHash::new(hash as u16);
    buf.extend_from_slice(&hash_alg.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        *object = BigEndian::read_u32(&slice);
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:80 - TPM2_SequenceUpdate
/// Update hash calculation in TPM
pub fn tpm_sequence_update(tpm: &TpmDev, locality: u32,
                           object: u32, buffer: Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_SEQUENCE_UPDATE as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // sequenceHandle: TpmIDhObject
    let sequence_handle = TpmIDhObject::new(object);
    buf.extend_from_slice(&sequence_handle.to_vec());

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16, 0 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // buffer: Tpm2BMaxBuffer
    let buffer_size: u16 = buffer.len() as u16;
    buf.extend_from_slice(&u16::to_be_bytes(buffer_size));
    buf.extend_from_slice(&buffer);

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Print response
    println!("postsend: {:x?}", buf);
    true
}

/// Table 3:82 - TPM2_SequenceComplete
/// Finalize hash calculation in TPM
pub fn tpm_sequence_complete(tpm: &TpmDev, locality: u32, object: u32, buffer: Vec<u8>, 
                             hash_size: &mut u16, hash: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_SEQUENCE_COMPLETE as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // sequenceHandle: TpmIDhObject
    let sequence_handle = TpmIDhObject::new(object);
    buf.extend_from_slice(&sequence_handle.to_vec());

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16, 0 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // buffer: Tpm2BMaxBuffer
    let buffer_size: u16 = buffer.len() as u16;
    buf.extend_from_slice(&u16::to_be_bytes(buffer_size));
    buf.extend_from_slice(&buffer);

    // hierarchy: TpmIRhHierarchy
    buf.extend_from_slice(&u32::to_be_bytes(TpmRH::TPM_RH_OWNER as u32));

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        let mut th = 0;
        let response_size = BigEndian::read_u32(&slice[th..(th + 4)]);
        th += 4;
        println!("response_size: {}", response_size as usize);
        *hash_size = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        println!("hash_size: {}", *hash_size as usize);
        hash.extend([0].repeat(*hash_size as usize));
        hash.copy_from_slice(&slice[th..(th + *hash_size as usize)]);
        th += *hash_size as usize;
        let ticket = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(ticket == TpmStructures::TPM_ST_HASHCHECK as u16);
        let hierarchy = BigEndian::read_u32(&slice[th..(th + 4)]);
        th += 4;
        assert!(hierarchy == TpmRH::TPM_RH_OWNER as u32);
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:62 - TPM2_Hash
/// Generic hash calculation in TPM when data size is known
pub fn tpm_hash(tpm: &TpmDev, locality: u32, hash: TpmAlgorithms, buffer: Vec<u8>, 
                hash_size: &mut u16, hash_val: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_HASH as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // buffer: TPM2B_MAXBUFFER
    let buffer_size: u16 = buffer.len() as u16;
    buf.extend_from_slice(&u16::to_be_bytes(buffer_size));
    buf.extend_from_slice(&buffer);

    // hashAlg: TPMI_ALGHASH
    let hash_alg = TpmIAlgHash::new(hash as u16);
    buf.extend_from_slice(&hash_alg.to_vec());

    // hierarchy: TPMI_RHHIERARCHY
    buf.extend_from_slice(&u32::to_be_bytes(TpmRH::TPM_RH_OWNER as u32));

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        let mut th = 0;
        *hash_size = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        println!("hash_size: {}", *hash_size as usize);
        hash_val.extend([0].repeat(*hash_size as usize));
        hash_val.copy_from_slice(&slice[th..(th + *hash_size as usize)]);
        th += *hash_size as usize;
        let ticket = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(ticket == TpmStructures::TPM_ST_HASHCHECK as u16);
        let hierarchy = BigEndian::read_u32(&slice[th..(th + 4)]);
        th += 4;
        assert!(hierarchy == TpmRH::TPM_RH_OWNER as u32);
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:164 - TPM2_PCR_CreatePrimary Command
/// Create Primary Key.
/// This includes Storate Root Keys and Attestation Identity Keys.
pub fn tpm_create_primary(tpm: &TpmDev, locality: u32,
                          pcr_idx: Option<usize>, unique_base: &[u8],
                          restricted: bool, decrypt: bool, sign: bool,
                          parent_handle: &mut u32, pubkey_size: &mut usize,
                          pubkey: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_CREATE_PRIMARY as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // primaryHandle: TPMI_RH_HIERARCHY
    buf.extend_from_slice(&u32::to_be_bytes(TpmRH::TPM_RH_OWNER as u32));

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16, 0 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // inSensitive: TPM2B_SENSITIVE_CREATE
    let in_sensitive = Tpm2BSensitiveCreate::new(
        TpmSSensitiveCreate::new( // sensitive: TPMS_SENSTIVE_CREATE
            Tpm2BAuth::new(Vec::<u8>::new()), // userAuth: TPM2B_AUTH
            Tpm2BSensitiveData::new(Vec::<u8>::new()) // data: TPM2B_SENSITIVE_DATA
        )
    );
    buf.extend_from_slice(&in_sensitive.to_vec());

    // publicArea.objectAttributes: TPMA_OBJECT
    let mut objectAttributes = TpmAObject(0);
    objectAttributes.set_sign(sign);
    objectAttributes.set_decrypt(decrypt);
    objectAttributes.set_restricted(restricted);
    objectAttributes.set_user_with_auth(true);
    objectAttributes.set_sensitive_data_origin(true);
    objectAttributes.set_fixed_parent(true);
    objectAttributes.set_fixed_tpm(true);

    // publicArea.authPolicy: TPM2B_DIGEST
    let auth_policy = Tpm2BDigest::new(Vec::<u8>::new());

    // symmetric.algorithm: TPMI_ALG_SYM_OBJECT
    let algorithm: TpmIAlgSymObject;
    // symmetric.aesKeybits: TPMU_SYM_KEY_BITS
    let aes_keybits: Option<TpmUSymKeyBits>;
    // symmetric.aesMode: TPMU_SYM_MODE
    let aes_mode: Option<TpmUSymMode>;
    /// If the asymmetric key is a restricted decryption key, the 'symmetric' field will be
    /// set to a supported symmetric algorithm, key size, and mode.
    /// If the asymmetric key is not a restricted decryption key, then the 'symmetric' field
    /// will be set to TPM_ALG_NULL.
    if restricted && decrypt {
        algorithm = TpmIAlgSymObject::new(TpmAlgorithms::TPM_ALG_AES as u16);
        let keybits: TpmIAesKeyBits = TpmIAesKeyBits::new(128 as u16);
        aes_keybits = Some(TpmUSymKeyBits::new(keybits));
        let mode: TpmIAlgSymMode = TpmIAlgSymMode::new(TpmAlgorithms::TPM_ALG_CFB as u16);
        aes_mode = Some(TpmUSymMode::new(mode));
    }
    else {
        algorithm = TpmIAlgSymObject::new(TpmAlgorithms::TPM_ALG_NULL as u16);
        aes_keybits = None;
        aes_mode = None;
    }
    // rsaParms.symmetric: TPMT_SYM_DEF_OBJECT
    let symmetric = TpmTSymDefObject::new(algorithm, aes_keybits, aes_mode);

    // rsaParms.scheme: TPMT_RSA_SCHEME
    let scheme: TpmTRsaScheme;
    /// If the asymmetric key is:
    /// - an unrestricted signing key, then TPM_ALG_RSAPSS, TPM_ALG_RSASSA, or TPM_ALG_NULL
    /// - a restricted signing key, then TPM_ALG_RSA_PSS, or TPM_ALG_RSASSA
    /// - an unrestricted decryption key, then TPM_ALG_RSAES, TPM_ALG_OAEP, or TPM_ALG_NULL
    /// - a restricted decryption key, then TPM_ALG_NULL
    if sign && !decrypt {
        scheme = TpmTRsaScheme::new(TpmAlgorithms::TPM_ALG_RSASSA as u16,
                                    Some(TpmAlgorithms::TPM_ALG_SHA256 as u16));
    }
    else {
        scheme = TpmTRsaScheme::new(TpmAlgorithms::TPM_ALG_NULL as u16, None);
    }

    // parameters.rsaParms: TPMS_RSA_PARMS
    let rsa_parms = Some(TpmSRsaParms::new(symmetric, scheme, 2048 as u16, 0 as u32));

    // publicArea.parameters: TPMU_PUBLIC_PARMS
    let parameters = TpmUPublicParms::new(TpmAlgorithms::TPM_ALG_RSA, None, None, rsa_parms);

    // publicArea.unique: TPMU_PUBLIC_ID
    let hash: Vec<u8> = Sha256::digest(unique_base).to_vec();
    let unique = Tpm2BPublicKeyRsa::new(hash);

    // inPublic.publicArea: TPMT_PUBLIC
    let public_area = TpmTPublic::new(
        TpmAlgorithms::TPM_ALG_RSA as u16,
        TpmAlgorithms::TPM_ALG_SHA256 as u16,
        objectAttributes.bit_range(31, 0),
        auth_policy,
        parameters,
        unique
    );

    // inPublic: TPM2B_PUBLIC
    let in_public = Tpm2BPublic::new(public_area);
    buf.extend_from_slice(&in_public.to_vec());

    // outsideInfo: TPM2B_DATA
    let outside_info = Tpm2BData::new(Vec::<u8>::new());
    buf.extend_from_slice(&outside_info.to_vec());

    // creationPcr: TPML_PCR_SELECTION
    match pcr_idx {
        Some(x) => {
            // pcrSelection: TPMS_PCR_SELECTION
            let mut pcr_select: Vec<u8>;
            pcr_select = Vec::with_capacity(TPM_PCR_SELECT_MIN);
            pcr_select.extend([0].repeat(TPM_PCR_SELECT_MIN));
            pcr_select[x >> 3] = 1 << (x & 0x7);
            let pcr_selection: TpmSPcrSelection = TpmSPcrSelection::new(
                TpmAlgorithms::TPM_ALG_SHA256 as u16, // hash_alg
                TPM_PCR_SELECT_MIN as u8, // size_of_select
                pcr_select // pcr_select
            );

            // pcrSelections: TPMS_PCR_SELECTION[]
            let count: u32 = 1;
            let mut pcr_selections: Vec<TpmSPcrSelection> = Vec::with_capacity(count as usize);
            pcr_selections.push(pcr_selection);

            // creationPcr: TPML_PCR_SELECTION
            let creation_pcr: TpmLPcrSelection = TpmLPcrSelection::new(
                count, // count,
                pcr_selections // TpmSPcrSelection
            );
            buf.extend_from_slice(&creation_pcr.to_vec());
        },
        None => {
            buf.extend_from_slice(&u32::to_be_bytes(0));
        },
    }

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        // objectHandle
        let mut th = 0;
        let object_handle: u32 = BigEndian::read_u32(&slice[th..(th + 4)]);
        *parent_handle = object_handle;
        th += 4;
        // bodySize
        th += 4; // let body_size: u32 = BigEndian::read_u32(&slice[th..(th + 4)]);
        // outPublic
        th += 2; // let size: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        // outPublic.publicArea
        let algtype: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(algtype == TpmAlgorithms::TPM_ALG_RSA as u16);
        let namealg: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(namealg == TpmAlgorithms::TPM_ALG_SHA256 as u16);
        th += 4; // let objectattributes: u32 = BigEndian::read_u32(&slice[th..(th + 4)]);
        th += 2; // let authpolicy_size: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        // outPublic.publicArea.paramers.rsaDetail.symmetric
        let algorithm: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        if sign {
            assert!(algorithm == TpmAlgorithms::TPM_ALG_NULL as u16);
        }
        else {
            assert!(algorithm == TpmAlgorithms::TPM_ALG_AES as u16);
            let keybits_aes_keysizesbits: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
            th += 2;
            assert!(keybits_aes_keysizesbits == 128 as u16);
            let mode_aes_mode: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
            th += 2;
            assert!(mode_aes_mode == TpmAlgorithms::TPM_ALG_CFB as u16);
        }
        let scheme_scheme: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        if sign && !decrypt {
            assert!(scheme_scheme == TpmAlgorithms::TPM_ALG_RSASSA as u16);
            let scheme_details: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
            th += 2;
            assert!(scheme_details == TpmAlgorithms::TPM_ALG_SHA256 as u16);
        }
        else {
            assert!(scheme_scheme == TpmAlgorithms::TPM_ALG_NULL as u16);
        }
        let keybits: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(keybits == 2048 as u16);
        let exponent: u32 = BigEndian::read_u32(&slice[th..(th + 4)]);
        th += 4;
        assert!(exponent == 0 as u32);
        // outPublic.publicArea.unique
        let rsa_size: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        *pubkey_size = rsa_size as usize;
        pubkey.extend([0].repeat(*pubkey_size));
        pubkey.copy_from_slice(&slice[th..(th + *pubkey_size)]);
        // Ignoring the rest of the parsing...
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:15 - TPM2_StartAuthSession Command
/// Start Authenticated Session and returns a session handle
pub fn tpm_start_auth_session(tpm: &TpmDev, locality: u32, session_type: TpmSE,
                              nonce: Vec<u8>, session_handle: &mut u32) -> bool {
    let mut buf: Vec<u8>;
    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_START_AUTH_SESSION as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // tpmKey: TPMI_DH_OBJECT
    let tpm_key = TpmIDhObject::new(TpmRH::TPM_RH_NULL as u32);
    buf.extend_from_slice(&tpm_key.to_vec());

    // bind: TPMI_DH_ENTITY
    let bind = TpmIDhEntity::new(TpmRH::TPM_RH_NULL as u32);
    buf.extend_from_slice(&bind.to_vec());

    // nonceCaller: TPM2B_NONCE
    let nonce_caller = Tpm2BNonce::new(nonce);
    buf.extend_from_slice(&nonce_caller.to_vec());

    // encryptedSalt: TPM2B_ENCRYPTED_SECRET
    let encrypted_salt = Tpm2BEncryptedSecret::new(Vec::<u8>::new());
    buf.extend_from_slice(&encrypted_salt.to_vec());

    // sessionType: TPM_SE (= u8)
    buf.extend_from_slice(&u8::to_be_bytes(session_type as u8));

    // symmetric: TPMT_SYM_DEF
    let algorithm = TpmIAlgSym::new(TpmAlgorithms::TPM_ALG_NULL as u16);
    let symmetric = TpmTSymDef::new(algorithm, None, None);
    buf.extend_from_slice(&symmetric.to_vec());

    // authHash: TPMI_ALG_HASH
    let auth_hash = TpmIAlgHash::new(TpmAlgorithms::TPM_ALG_SHA256 as u16);
    buf.extend_from_slice(&auth_hash.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        // sessionHandle
        let mut th = 0;
        *session_handle = BigEndian::read_u32(&slice[th..(th + 4)]);
    } else {
        println!("StartAuthSession Failed");
        return false;
    }
    true
}

/// Table 3:132 - TPM2_PolicyPCR Command
/// Bind a policy to a particular PCR
pub fn tpm_policy_pcr(tpm: &TpmDev, locality: u32, session_handle: u32,
                      digest: Vec<u8>, pcr_idx: usize) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_POLICY_PCR as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // policySession: TPMI_SH_POLICY
    let policy_session = TpmIShPolicy::new(session_handle);
    buf.extend_from_slice(&policy_session.to_vec());

    // pcrDigest: TPM2B_DIGEST
    let pcr_digest = Tpm2BDigest::new(digest);
    buf.extend_from_slice(&pcr_digest.to_vec());

    // pcrSelection: TPMS_PCR_SELECTION
    let hash: u16 = TpmAlgorithms::TPM_ALG_SHA256 as u16;
    let mut pcr_select: Vec<u8>;
    pcr_select = Vec::with_capacity(TPM_PCR_SELECT_MIN);
    pcr_select.extend([0].repeat(TPM_PCR_SELECT_MIN));
    pcr_select[pcr_idx >> 3] = 1 << (pcr_idx & 0x7);
    let pcr_selection: TpmSPcrSelection = TpmSPcrSelection::new(
        hash, // hash
        TPM_PCR_SELECT_MIN as u8, // size_of_select
        pcr_select // pcr_select
    );

    // pcrSelections: TPMS_PCR_SELECTION[]
    let count: u32 = 1;
    let mut pcr_selections: Vec<TpmSPcrSelection> = Vec::with_capacity(count as usize);
    pcr_selections.push(pcr_selection);

    // pcrs: TPML_PCR_SELECTION
    let pcrs: TpmLPcrSelection = TpmLPcrSelection::new(
        count, // count,
        pcr_selections // TPMS_PCR_SELECTION[]
    );
    buf.extend_from_slice(&pcrs.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Print response
    println!("postsend: {:x?}", buf);
    true
}

/// Table 3:156 - TPM2_PolicyGetDigest Command
/// Get Policy digest from current policy
pub fn tpm_policy_get_digest(tpm: &TpmDev, locality: u32, session_handle: u32,
                             policy_digest: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_POLICY_GET_DIGEST as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // policySession: TPMI_SH_POLICY
    let policy_session = TpmIShPolicy::new(session_handle);
    buf.extend_from_slice(&policy_session.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        // policyDigest
        let mut th = 0;
        let size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        policy_digest.clear();
        policy_digest.extend([0].repeat(size));
        policy_digest.copy_from_slice(&slice[th..(th + size)]);
    } else {
        println!("PolicyGetDigest Failed");
        return false;
    }
    true
}

pub fn create_symcipher(policy: Vec<u8>) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    // inSensitive: Tpm2BSensitiveCreate
    let user_auth = Tpm2BAuth::new(Vec::<u8>::new());
    let data = Tpm2BSensitiveData::new(Vec::<u8>::new());
    let sensitive = TpmSSensitiveCreate::new(user_auth, data);
    let sensitive_create = Tpm2BSensitiveCreate::new(sensitive);
    buf.extend_from_slice(&sensitive_create.to_vec());
    // inPublic: Tpm2BPublic
    let mut objectAttributes = TpmAObject(0);
    objectAttributes.set_decrypt(true);
    objectAttributes.set_restricted(true);
    objectAttributes.set_sensitive_data_origin(true);
    objectAttributes.set_fixed_parent(true);
    objectAttributes.set_fixed_tpm(true);
    let auth_policy: Tpm2BDigest = Tpm2BDigest::new(policy);
    let algorithm = TpmIAlgSymObject::new(TpmAlgorithms::TPM_ALG_AES as u16);
    let keybits: TpmIAesKeyBits = TpmIAesKeyBits::new(128 as u16);
    let aes_keybits = Some(TpmUSymKeyBits::new(keybits));
    let mode: TpmIAlgSymMode = TpmIAlgSymMode::new(TpmAlgorithms::TPM_ALG_CFB as u16);
    let aes_mode = Some(TpmUSymMode::new(mode));
    let symmetric = TpmTSymDefObject::new(algorithm, aes_keybits, aes_mode);
    let symcipher_parms = Some(TpmSSymcipherParms::new(symmetric));
    let parameters = TpmUPublicParms::new(TpmAlgorithms::TPM_ALG_SYMCIPHER, None,
                                          symcipher_parms, None);
    let unique: Tpm2BPublicKeyRsa = Tpm2BPublicKeyRsa::new(Vec::<u8>::new());
    let public_area: TpmTPublic = TpmTPublic::new(TpmAlgorithms::TPM_ALG_SYMCIPHER as u16,
                                                  TpmAlgorithms::TPM_ALG_SHA256 as u16,
                                                  objectAttributes.bit_range(31, 0),
                                                  auth_policy,
                                                  parameters,
                                                  unique);
    let in_public: Tpm2BPublic = Tpm2BPublic::new(public_area);
    buf.extend_from_slice(&in_public.to_vec());
    buf
}

/// Table 3:19 - TPM2_Create Command
/// Create child key
pub fn tpm_create(tpm: &TpmDev, locality: u32, pcr_idx: Option<usize>,
                  parent_handle: u32, policy: Vec<u8>, sensitive_data: Vec<u8>,
                  restricted: bool, decrypt: bool, sign: bool,
                  out_private: &mut Vec<u8>, out_public: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_CREATE as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // parentHandle: TPMI_DH_PARENT
    buf.extend_from_slice(&u32::to_be_bytes(parent_handle));

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16, 0 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // inSensitive: TPM2B_SENSITIVE_CREATE
    let in_sensitive = Tpm2BSensitiveCreate::new(
        TpmSSensitiveCreate::new( // sensitive: TPMS_SENSTIVE_CREATE
            Tpm2BAuth::new(Vec::<u8>::new()), // userAuth: TPM2B_AUTH
            Tpm2BSensitiveData::new(sensitive_data) // data: TPM2B_SENSITIVE_DATA
        )
    );
    buf.extend_from_slice(&in_sensitive.to_vec());

    // publicArea.objectAttributes: TPMA_OBJECT
    let mut objectAttributes = TpmAObject(0);
    objectAttributes.set_sign(sign);
    objectAttributes.set_decrypt(decrypt);
    objectAttributes.set_restricted(restricted);
    objectAttributes.set_fixed_parent(true);
    objectAttributes.set_fixed_tpm(true);

    // publicArea.authPolicy: TPM2B_DIGEST
    let auth_policy: Tpm2BDigest = Tpm2BDigest::new(policy);

    // scheme.details: TPMU_SCHEME_KEYEDHASH
    let details = TpmUSchemeKeyedHash::new(TpmAlgorithms::TPM_ALG_NULL, None, None);
    // keyedhashParms.scheme: TPMT_KEYEDHASH_SCHEME
    let scheme = TpmTKeyedhashScheme::new(TpmAlgorithms::TPM_ALG_NULL as u16, details);
    // parameters.keyedhashParms: TPMS_KEYEDHASH_PARMS
    let keyedhash_parms = Some(TpmSKeyedhashParms::new(scheme));
    // publicArea.parameters: TPMU_PUBLIC_PARMS
    let parameters = TpmUPublicParms::new(TpmAlgorithms::TPM_ALG_KEYEDHASH, keyedhash_parms,
                                          None, None);
    
    // publicArea.unique: TPMU_PUBLIC_ID
    let unique = Tpm2BPublicKeyRsa::new(Vec::<u8>::new());

    // inPublic.publicArea: TPMT_PUBLIC
    let public_area = TpmTPublic::new(
        TpmAlgorithms::TPM_ALG_KEYEDHASH as u16,
        TpmAlgorithms::TPM_ALG_SHA256 as u16,
        objectAttributes.bit_range(31, 0),
        auth_policy,
        parameters,
        unique
    );

    // inPublic: TPM2B_PUBLIC
    let in_public: Tpm2BPublic = Tpm2BPublic::new(public_area);
    buf.extend_from_slice(&in_public.to_vec());

    // outsideInfo: Tpm2BData
    let outside_info: Tpm2BData = Tpm2BData::new(Vec::<u8>::new());
    buf.extend_from_slice(&outside_info.to_vec());

    // creationPcr: TpmLPcrSelection
    match pcr_idx {
        Some(x) => {
            // pcrSelection: TPMS_PCR_SELECTION
            let mut pcr_select: Vec<u8>;
            pcr_select = Vec::with_capacity(TPM_PCR_SELECT_MIN);
            pcr_select.extend([0].repeat(TPM_PCR_SELECT_MIN));
            pcr_select[x >> 3] = 1 << (x & 0x7);
            let pcr_selection: TpmSPcrSelection = TpmSPcrSelection::new(
                TpmAlgorithms::TPM_ALG_SHA256 as u16, // hash_alg
                TPM_PCR_SELECT_MIN as u8, // size_of_select
                pcr_select // pcr_select
            );

            // pcrSelections: TPMS_PCR_SELECTION[]
            let count: u32 = 1;
            let mut pcr_selections: Vec<TpmSPcrSelection> = Vec::with_capacity(count as usize);
            pcr_selections.push(pcr_selection);

            // creationPcr: TPML_PCR_SELECTION
            let creation_pcr: TpmLPcrSelection = TpmLPcrSelection::new(
                count, // count,
                pcr_selections // TpmSPcrSelection
            );
            buf.extend_from_slice(&creation_pcr.to_vec());
        },
        None => {
            buf.extend_from_slice(&u32::to_be_bytes(0));
        },
    }
    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        let mut th = 0;
        th += 4; // Length of entire response body
        // outPrivate
        let out_private_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        out_private.clear();
        out_private.extend([0].repeat(out_private_size + 2));
        out_private.copy_from_slice(&slice[th..(th + out_private_size + 2)]);
        th =  th + out_private_size + 2;
        // outPublic
        let out_public_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        out_public.clear();
        out_public.extend([0].repeat(out_public_size + 2));
        out_public.copy_from_slice(&slice[th..(th + out_public_size + 2)]);
        th =  th + out_public_size + 2;
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:21 - TPM2_Load Command
/// Load objects into the TPM.
/// The TPM2B_PUBLIC and TPM2B_PRIVATE objects created by the TPM2_Create command 
/// are to be loaded.
pub fn tpm_load(tpm: &TpmDev, locality: u32, parent_handle: u32,
                in_private: Vec<u8>, in_public: Vec<u8>, item_handle: &mut u32) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_LOAD as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // parentHandle: TPMI_DH_PARENT
    buf.extend_from_slice(&u32::to_be_bytes(parent_handle));

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16, 0 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // inPrivate
    buf.extend_from_slice(&in_private);

    // inPublic
    buf.extend_from_slice(&in_public);

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        let mut th = 0;
        *item_handle = BigEndian::read_u32(&slice[th..(th + 4)]);
        th += 4;
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:31 - TPM2_Unseal Command
/// Unseal data sealed via TPM_CC_CREATE
pub fn tpm_unseal(tpm: &TpmDev, locality: u32, session_handle: u32, item_handle: u32,
                  out_data: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_UNSEAL as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // itemHandle: TPMI_DH_OBJECT
    buf.extend_from_slice(&u32::to_be_bytes(item_handle));

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(session_handle, 0 as u16, 1 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        let mut th = 0;
        th += 4; // Length of entire response body
        let out_data_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        out_data.extend([0].repeat(out_data_size));
        out_data.copy_from_slice(&slice[th..(th + out_data_size)]);
        th += out_data_size;
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:90 - TPM2_Quote
/// Generate Quote.
/// Since the communication channel between the process and the TPM is untrusted,
/// TPM2_Quote should be the command to retreive PCR values, not TPM2_PCR_Read
pub fn tpm_quote(tpm: &TpmDev, locality: u32, handle: u32, hash: u16,
                 nonce: Vec<u8>, pcr_idxs: Vec<usize>,
                 out_pcr_digest: &mut Vec<u8>, out_sig: &mut Vec<u8>) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_QUOTE as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // signHandle: TPMI_DH_OBJECT
    let sign_handle = TpmIDhObject::new(handle);
    buf.extend_from_slice(&sign_handle.to_vec());

    // handle (required whenever header.tag is TPM_ST_SESSIONS)
    let tpm_handle = TpmHandle::new(TpmRH::TPM_RS_PW as u32, 0 as u16, 1 as u8, 0 as u16);
    buf.extend_from_slice(&tpm_handle.to_vec());

    // qualifyingData: TPM2B_DATA
    let qualifying_data = Tpm2BData::new(nonce);
    buf.extend_from_slice(&qualifying_data.to_vec());

    // inScheme.scheme: TPMI_ALG_SIG_SCHEME
    let scheme = TpmIAlgSigScheme::new(TpmAlgorithms::TPM_ALG_NULL as u16);
    // inScheme.details: TPMU_SIG_SCHEME
    let details = TpmUSigScheme::new(TpmAlgorithms::TPM_ALG_NULL, None, None, None);
    // inScheme: TPMT_SIG_SCHEME
    let in_scheme = TpmTSigScheme::new(scheme, details);
    buf.extend_from_slice(&in_scheme.to_vec());

    // pcrSelections: TPMS_PCR_SELECTION[]
    let count: u32 = pcr_idxs.len() as u32;
    let mut s_pcr_selections: Vec<TpmSPcrSelection> = Vec::with_capacity(count as usize);
    for pcr_idx in pcr_idxs {
        // pcrSelection: TPMS_PCR_SELECTION
        let mut pcr_select: Vec<u8>;
        pcr_select = Vec::with_capacity(TPM_PCR_SELECT_MIN);
        pcr_select.extend([0].repeat(TPM_PCR_SELECT_MIN));
        pcr_select[pcr_idx >> 3] = 1 << (pcr_idx & 0x7);
        let s_pcr_selection: TpmSPcrSelection = TpmSPcrSelection::new(
            hash, // hash
            TPM_PCR_SELECT_MIN as u8, // size_of_select
            pcr_select // pcr_select
        );
        s_pcr_selections.push(s_pcr_selection);
    }
    // PCRselect: TPML_PCR_SELECTION
    let l_pcr_selection: TpmLPcrSelection = TpmLPcrSelection::new(
        count, // count,
        s_pcr_selections // TpmSPcrSelection
    );
    buf.extend_from_slice(&l_pcr_selection.to_vec());

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Parse response
    println!("postsend: {:x?}", buf);
    if buf.len() > 0 {
        let mut slice = buf.as_slice();
        let mut th = 0;
        th += 4; // Length of entire response body
        let quoted_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        let quoted_magic: u32 = BigEndian::read_u32(&slice[th..(th + 4)]) as u32;
        th += 4;
        assert!(quoted_magic == 0xff544347);
        let quoted_type: u16 = BigEndian::read_u16(&slice[th..(th + 2)]) as u16;
        th += 2;
        assert!(quoted_type == TpmStructures::TPM_ST_ATTEST_QUOTE as u16);
        let qualified_signer_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        th += qualified_signer_size;
        let extra_data_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        th += extra_data_size;
        th += 8; // clockInfo clock
        th += 4; // clockInfo resetCount
        th += 4; // clockInfo restartCount
        th += 1; // clockInfo safe
        th += 8; // firmwareVersion
        let pcr_select_count: usize = BigEndian::read_u32(&slice[th..(th + 4)]) as usize;
        th += 4;
        for _ in 0..pcr_select_count {
            th += 2; // pcrSelections hash
            th += 1; // pcrSelections sizeofSelect
            th += 3; // pcrSelections pcrSelect
        }
        let pcr_digest_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        out_pcr_digest.extend([0].repeat(pcr_digest_size));
        out_pcr_digest.copy_from_slice(&slice[th..(th + pcr_digest_size)]);
        th += pcr_digest_size;
        let sig_alg: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(sig_alg == TpmAlgorithms::TPM_ALG_RSASSA as u16);
        let rsassa_hash: u16 = BigEndian::read_u16(&slice[th..(th + 2)]);
        th += 2;
        assert!(rsassa_hash == TpmAlgorithms::TPM_ALG_SHA256 as u16);
        let sig_size: usize = BigEndian::read_u16(&slice[th..(th + 2)]) as usize;
        th += 2;
        out_sig.extend([0].repeat(sig_size));
        out_sig.copy_from_slice(&slice[th..(th + sig_size)]);
        th += sig_size;
    } else {
        println!("Didn't receive any response from TPM!");
        return false;
    }
    true
}

/// Table 3:198 - TPM2_FlushContext Command
/// Remove loaded objects, sequence objects, and/or sessions from TPM memory
pub fn tpm_flush_context(tpm: &TpmDev, locality: u32, flush_handle: u32) -> bool {
    let mut buf: Vec<u8>;

    // header
    let hdr = TpmHeader::new(
        TpmStructures::TPM_ST_NO_SESSIONS as u16,
        0 as u32,
        Tpm2Commands::TPM2_CC_FLUSH_CONTEXT as u32
    );
    buf = TpmHeader::to_vec(&hdr);

    // flushHandle: TPMI_DH_CONTEXT
    buf.extend_from_slice(&u32::to_be_bytes(flush_handle));

    // Change the size of command in header
    buf.splice(2..6, (buf.len() as u32).to_be_bytes().into_iter().cloned());

    // Send command
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, locality, &mut buf);

    // Print response
    println!("postsend: {:x?}", buf);
    true
}
