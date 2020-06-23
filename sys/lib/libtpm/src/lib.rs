#![no_std]

extern crate alloc;
extern crate malloc;
extern crate byteorder;

#[macro_use]
extern crate bitfield;

mod regs;

use alloc::vec::Vec;
use bitfield::BitRange;
use console::{print, println};
use libtime::sys_ns_loopsleep;
use usr::tpm::{TpmDev, TpmRegs};
pub use regs::*;
use byteorder::{ByteOrder, BigEndian};

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

fn tpm_buf_append_u16(buf: &mut Vec<u8>, data: u16) {
    buf.extend_from_slice(&u16::to_be_bytes(data));
}

fn tpm_buf_append_u32(buf: &mut Vec<u8>, data: u32) {
    buf.extend_from_slice(&u32::to_be_bytes(data));
}

fn tpm_buf_append(buf: &mut Vec<u8>, data: &Vec <u8>) {
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
            return true;
        }
    }

    return false;
}

/// Explicitly giveup locality. This may not be useful if there is only a single process/user using
/// TPM in an OS. In multi-user scenario, this is more applicable.
fn relinquish_locality(tpm: &dyn TpmDev, locality: u32) -> bool {
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

/// Requests the TPM to switch to the locality we choose and wait for TPM to acknowledge our
/// request
fn request_locality(tpm: &dyn TpmDev, locality: u32) -> bool {
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
pub fn tpm_get_burst(tpm: &TpmDev) -> u16 {
    let reg_sts = tpm.read_u32(0, TpmRegs::TPM_STS);
    println!("{:x?}", u32::to_le_bytes(reg_sts));
    (reg_sts >> 8) as u16 & 0xFFFF
}

/// Busy-wait in a loop for a particular status flag to be set
fn wait_for_status_flag(tpm: &TpmDev, locality: u32, flag: u8, timeout_ms: usize) -> bool {

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
fn tpm_write_data(tpm: &dyn TpmDev, locality: u32, data: &[u8]) -> usize {
    let burst_count = tpm_get_burst(tpm) as usize;

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
fn is_data_available(tpm: &dyn TpmDev, locality: u32) -> bool {
    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
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
fn tpm_read_data(tpm: &dyn TpmDev, locality: u32, data: &mut [u8]) -> usize {
    let reg_sts = tpm.read_u8(0, TpmRegs::TPM_STS);
    let mut status = TpmStatus(reg_sts);

    status.set_sts_valid(true);
    status.set_data_avail(true);

    if !wait_for_status_flag(tpm, locality, status.bit_range(7, 0), TIMEOUT_C) {
        return 0;
    }

    let mut data = data;
    let mut size = 0;
    let mut burst_count = 0;

    loop {
        burst_count = tpm_get_burst(tpm) as usize;

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
fn tpm_recv_data(tpm: &TpmDev, locality: u32, buf: &mut Vec<u8>) -> usize {
    let size = buf.len();

    buf.clear();
    buf.extend([0].repeat(TPM_HEADER_SIZE));

    tpm_read_data(tpm, locality, buf.as_mut_slice());

    let hdr = TpmHeader::from_vec(buf);

    if hdr.length as usize <= size {
        println!("Expected len {} > buf size {}", hdr.length, size);
        return 0;
    }
    // Check whether TPM Return Code is TPM_SUCCESS
    if hdr.ordinal != (Tpm2ReturnCodes::TPM2_RC_SUCCESS as u32) {
        println!("TPM returned with error {}", hdr.ordinal);
        return 0;
    }

    buf.clear();
    buf.extend([0].repeat(hdr.length as usize - TPM_HEADER_SIZE));

    let ret = tpm_read_data(tpm, locality, buf.as_mut_slice());

    tpm_send_command_ready!(tpm, locality);

    return ret;
}

/// Wrapper for `tpm_write_data`
/// This function waits for TPM to be in a state to accept commands before writing data to FIFO.
fn tpm_send_data(tpm: &TpmDev, locality: u32, buf: &mut Vec<u8>) -> usize {
    let mut reg_sts = tpm.read_u8(locality, TpmRegs::TPM_STS);
    let mut status = TpmStatus(reg_sts);

    if !status.command_ready() {
        // If TPM is not ready, make it ready
        status.set_command_ready(true);
        tpm.write_u8(locality, TpmRegs::TPM_STS, status.bit_range(7, 0));

        if !wait_for_status_flag(tpm, locality, status.bit_range(7, 0), TIMEOUT_B) {
            return 0;
        }
    }

    return tpm_write_data(tpm, locality, buf.as_slice());
}

/// Transmit command to a TPM.
/// This function does a bi-directional communication with TPM.
/// First, it sends a command with headers
/// If successful, try to read the response buffer from TPM
fn tpm_transmit_cmd(tpm: &TpmDev, locality: u32, buf: &mut Vec<u8>) {
    let hdr: TpmHeader = TpmHeader::from_vec(&buf);

    println!("tpm_transmit_cmd len {} ord {:x}", hdr.length, hdr.ordinal);

    let tx_bytes = tpm_send_data(tpm, locality, buf);

    println!("Transmitted {} bytes", tx_bytes);

    let rx_bytes = tpm_recv_data(tpm, locality, buf);

    println!("Received {} bytes", rx_bytes);
}

/// Get a random number from TPM. 
/// `num_octets` represents the length of the random number in bytes
pub fn tpm_get_random(tpm: &TpmDev, num_octets: usize) -> bool {
    let mut buf: Vec<u8>;
    let data_size = 2; // bytesRequested: u16 from TCG specification
    let command_len = TPM_HEADER_SIZE + data_size;
    let mut hdr: TpmHeader = TpmHeader::new(
        Tpm2Structures::TPM2_ST_NO_SESSIONS as u16,
        command_len as u32,
        Tpm2Commands::TPM2_CC_GET_RANDOM as u32
    );
    buf = TpmHeader::to_vec(&hdr);
    buf.extend_from_slice(&(num_octets as u16).to_be_bytes());
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, 0, &mut buf);
    println!("postsend: {:x?}", buf);
    true
}

/// Read a PCR register
pub fn tpm_pcr_read(tpm: &TpmDev, pcr_idx: usize, hash: TpmAlgorithms, digest_size: &mut u16, digest: &mut Vec<u8>) -> bool {
    // Size of individual parameters: 
    // count: u32, 
    // hash: u16, 
    // sizeOfSelection: u8, 
    // pcrSelection: TPM_PCR_SELECT_MIN (= 3) Bytes
    let mut buf: Vec<u8>;
    let mut pcr_select: Vec<u8>;
    pcr_select = Vec::with_capacity(TPM_PCR_SELECT_MIN);
    pcr_select.extend([0].repeat(TPM_PCR_SELECT_MIN));
    pcr_select[pcr_idx >> 3] = 1 << (pcr_idx & 0x7);
    let data_size = 10;
    let command_len = TPM_HEADER_SIZE + data_size;
    let mut hdr: TpmHeader = TpmHeader::new(
        Tpm2Structures::TPM2_ST_NO_SESSIONS as u16,
        command_len as u32,
        Tpm2Commands::TPM2_CC_PCR_READ as u32
    );
    buf = TpmHeader::to_vec(&hdr);
    buf.extend_from_slice(&(1 as u32).to_be_bytes()); // count
    buf.extend_from_slice(&(hash as u16).to_be_bytes()); // hash
    buf.extend_from_slice(&(TPM_PCR_SELECT_MIN as u8).to_be_bytes()); // sizeOfSelection
    buf.extend_from_slice(&pcr_select); // pcr_select
    println!("presend: {:x?}", buf);
    tpm_transmit_cmd(tpm, 0, &mut buf);
    println!("postsend: {:x?}", buf);
    let mut slice = buf.as_slice();
    *digest_size = BigEndian::read_u16(&slice[18..20]);
    println!("digest_size: {}", digest_size);
    digest.extend([0].repeat(*digest_size as usize));
    digest.copy_from_slice(&slice[20..(20 + *digest_size as usize)]);
    true
}
