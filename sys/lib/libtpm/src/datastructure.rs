// Implementation of TPM datastructures
// Taken from TPM2.0 Specification Part 2
// (https://trustedcomputinggroup.org/wp-content/uploads/TCG_TPM2_r1p59_Part2_Structures_pub.pdf)

use alloc::vec::Vec;
use core::mem;
use byteorder::{ByteOrder, BigEndian};

pub struct TpmSPcrSelection {
    pub hash:           u16,
    pub size_of_select: u8,
    pub pcr_select:     Vec<u8>,
}

impl TpmSPcrSelection {
    pub fn new(hash: u16, size_of_select: u8, pcr_select: Vec<u8>) -> Self {
        Self {
            hash: hash.swap_bytes().to_be(),
            size_of_select: size_of_select.swap_bytes().to_be(),
            pcr_select: pcr_select,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + mem::size_of::<u8>() * (1 + self.size_of_select as usize);
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.hash));
        buf.extend_from_slice(&u8::to_be_bytes(self.size_of_select));
        buf.extend_from_slice(&self.pcr_select);
        buf
    }
}

pub struct TpmLPcrSelection {
    pub count: u32,
    pub pcr_selections: Vec<TpmSPcrSelection>,
}

impl TpmLPcrSelection {
    pub fn new(count: u32, pcr_selections: Vec<TpmSPcrSelection>) -> Self {
        Self {
            count: count.swap_bytes().to_be(),
            pcr_selections: pcr_selections,
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = mem::size_of::<u32>();
        for s in self.pcr_selections.iter() {
            ret = ret + s.size();
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(self.count));
        for s in self.pcr_selections.iter() {
            buf.extend_from_slice(&s.to_vec());
        }
        buf
    }
}

#[repr(packed)]
pub struct TpmPcrHandle {
    pub handle: u32,
    pub nonce_size: u16,
    pub attributes: u8,
    pub auth_size: u16,
}

impl TpmPcrHandle {
    pub fn new(handle: u32, nonce_size: u16, attributes: u8, auth_size: u16) -> Self {
        Self {
            handle: handle.swap_bytes().to_be(),
            nonce_size: nonce_size.swap_bytes().to_be(),
            attributes: attributes.swap_bytes().to_be(),
            auth_size: auth_size.swap_bytes().to_be(),
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = mem::size_of::<u32>() + mem::size_of::<TpmPcrHandle>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(mem::size_of::<TpmPcrHandle>() as u32));
        buf.extend_from_slice(&u32::to_be_bytes(self.handle));
        buf.extend_from_slice(&u16::to_be_bytes(self.nonce_size));
        buf.extend_from_slice(&u8::to_be_bytes(self.attributes));
        buf.extend_from_slice(&u16::to_be_bytes(self.auth_size));
        buf
    }
}

pub struct TpmIDhPcr {
    pub pcr_idx: u32,
    pub pcr_handle: TpmPcrHandle,
}

impl TpmIDhPcr {
    pub fn new(pcr_idx: u32, pcr_handle: TpmPcrHandle) -> Self {
        Self {
            pcr_idx: pcr_idx.swap_bytes().to_be(),
            pcr_handle: pcr_handle,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u32>() + self.pcr_handle.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(self.pcr_idx));
        buf.extend_from_slice(&TpmPcrHandle::to_vec(&self.pcr_handle));
        buf
    }
}

pub struct TpmDigest {
    pub hash: u16,
    pub digest: Vec<u8>,
}

impl TpmDigest {
    pub fn new(hash: u16, digest: Vec<u8>) -> Self {
        Self {
            hash: hash.swap_bytes().to_be(),
            digest: digest,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + self.digest.len() * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.hash));
        buf.extend_from_slice(&self.digest);
        buf
    }
}

pub struct TpmLDigestValues {
    pub count: u32,
    pub digests: Vec<TpmDigest>,
}

impl TpmLDigestValues {
    pub fn new(count: u32, digests: Vec<TpmDigest>) -> Self {
        Self {
            count: count.swap_bytes().to_be(),
            digests: digests,
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = mem::size_of::<u32>();
        for digest in self.digests.iter() {
            ret = ret + digest.size();
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(self.count));
        for digest in self.digests.iter() {
            buf.extend_from_slice(&digest.to_vec());
        }
        buf
    }
}
