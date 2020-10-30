/*
This library implements TPM datastructures.
Please refer to the individual specification shown in the specification
for more detail.
This library implements the specification proposed in the following link:
https://trustedcomputinggroup.org/wp-content/uploads/TCG_TPM2_r1p59_Part2_Structures_pub.pdf
*/

use alloc::vec::Vec;
use core::mem;
use byteorder::{ByteOrder, BigEndian};

pub use crate::regs::*;

/// TpmHandle is required when tag of a command or response is
/// TPM_ST_SESSIONS (c.f., Part 3, Section 4.4)
#[repr(packed)]
pub struct TpmHandle {
    pub handle: u32,
    pub nonce_size: u16,
    pub attributes: u8,
    pub auth_size: u16,
}

impl TpmHandle {
    pub fn new(handle: u32, nonce_size: u16, attributes: u8, auth_size: u16) -> Self {
        Self {
            handle: handle,
            nonce_size: nonce_size,
            attributes: attributes,
            auth_size: auth_size,
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = mem::size_of::<u32>() + mem::size_of::<TpmHandle>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(mem::size_of::<TpmHandle>() as u32));
        buf.extend_from_slice(&u32::to_be_bytes(self.handle));
        buf.extend_from_slice(&u16::to_be_bytes(self.nonce_size));
        buf.extend_from_slice(&u8::to_be_bytes(self.attributes));
        buf.extend_from_slice(&u16::to_be_bytes(self.auth_size));
        buf
    }
}

// Table 3:93 - TPMS_PCR_SELECTION
pub struct TpmSPcrSelection {
    pub hash_alg:           u16,
    pub size_of_select: u8,
    pub pcr_select:     Vec<u8>,
}

impl TpmSPcrSelection {
    pub fn new(hash_alg: u16, size_of_select: u8, pcr_select: Vec<u8>) -> Self {
        Self {
            hash_alg: hash_alg,
            size_of_select: size_of_select,
            pcr_select: pcr_select,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + mem::size_of::<u8>() * (1 + self.size_of_select as usize);
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.hash_alg));
        buf.extend_from_slice(&u8::to_be_bytes(self.size_of_select));
        buf.extend_from_slice(&self.pcr_select);
        buf
    }
}

// Table 3:111 - TPML_PCR_SELECTION
pub struct TpmLPcrSelection {
    pub count: u32,
    pub pcr_selections: Vec<TpmSPcrSelection>,
}

impl TpmLPcrSelection {
    pub fn new(count: u32, pcr_selections: Vec<TpmSPcrSelection>) -> Self {
        Self {
            count: count,
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

// Table 3:45 - TPMI_DH_PCR
pub struct TpmIDhPcr {
    pub pcr_idx: u32,
    pub pcr_handle: TpmHandle,
}

impl TpmIDhPcr {
    pub fn new(pcr_idx: u32, pcr_handle: TpmHandle) -> Self {
        Self {
            pcr_idx: pcr_idx,
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
        buf.extend_from_slice(&TpmHandle::to_vec(&self.pcr_handle));
        buf
    }
}

// Table 3:79 - TPMT_HA
pub use usr::tpm::TpmTHa;

// Table 3:110 - TPM2B_DIGEST_VALUES
pub struct TpmLDigestValues {
    pub count: u32,
    pub digests: Vec<TpmTHa>,
}

impl TpmLDigestValues {
    pub fn new(count: u32, digests: Vec<TpmTHa>) -> Self {
        Self {
            count: count,
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

// Table 3:80 - TPM2B_DIGEST
pub struct Tpm2BDigest {
    pub size: u16,
    pub buffer: Vec<u8>,
}

impl Tpm2BDigest {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            size: buffer.len() as u16,
            buffer: buffer,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.buffer);
        }
        buf
    }
}

// Table 3:81 - TPM2B_DIGEST
pub struct Tpm2BData {
    pub size: u16,
    pub buffer: Vec<u8>,
}

impl Tpm2BData {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            size: buffer.len() as u16,
            buffer: buffer,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.buffer);
        }
        buf
    }
}

// Table 3:83 - TPM2B_AUTH
pub struct Tpm2BAuth {
    pub size: u16,
    pub buffer: Vec<u8>,
}

impl Tpm2BAuth {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            size: buffer.len() as u16,
            buffer: buffer,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.buffer);
        }
        buf
    }
}

// Table 3:148 - TPM2B_SENSITIVE_DATA
pub struct Tpm2BSensitiveData {
    pub size: u16,
    pub buffer: Vec<u8>,
}

impl Tpm2BSensitiveData {
    pub fn new(buffer: Vec<u8>) -> Self {
        if buffer.len() < 128 {
            Self {
                size: buffer.len() as u16,
                buffer: buffer,
            }
        } else {
            Self {
                size: 0 as u16,
                buffer: Vec::new(),
            }
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.buffer);
        }
        buf
    }
}

// Table 3:149 - TPMS_SENSITIVE_CREATE
pub struct TpmSSensitiveCreate {
    pub user_auth: Tpm2BAuth,
    pub data: Tpm2BSensitiveData,
}

impl TpmSSensitiveCreate {
    pub fn new(user_auth: Tpm2BAuth, data: Tpm2BSensitiveData) -> Self {
        Self {
            user_auth: user_auth,
            data: data,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.user_auth.size() + self.data.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.user_auth.to_vec());
        buf.extend_from_slice(&self.data.to_vec());
        buf
    }
}

// Table 3:147 - TPM2B_SENSITIVE_CREATE
pub struct Tpm2BSensitiveCreate {
    pub size: u16,
    pub sensitive: TpmSSensitiveCreate,
}

impl Tpm2BSensitiveCreate {
    pub fn new(sensitive: TpmSSensitiveCreate) -> Self {
        Self {
            size: sensitive.size() as u16,
            sensitive: sensitive,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + self.sensitive.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size() > 0 {
            buf.extend_from_slice(&self.sensitive.to_vec());
        }
        buf
    }
}

// Table 3:151 - TPMS_SCHEME_HASH
pub struct TpmSSchemeHash {
    pub hash_alg: TpmIAlgHash,
}

impl TpmSSchemeHash {
    pub fn new(hash_alg: TpmIAlgHash) -> Self {
        Self {
            hash_alg: hash_alg,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.hash_alg.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.hash_alg.to_vec());
        buf
    }
}

// Table 3:154 - TPMS_SCHEME_HMAC
pub struct TpmSSchemeHmac {
    pub hash_alg: TpmIAlgHash,
}

impl TpmSSchemeHmac {
    pub fn new(hash_alg: TpmIAlgHash) -> Self {
        Self {
            hash_alg: hash_alg,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.hash_alg.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.hash_alg.to_vec());
        buf
    }
}

// Table 3:155 - TPMS_SCHEME_XOR
pub struct TpmSSchemeXor {
    pub hash_alg: TpmIAlgHash,
    pub kdf: TpmIAlgKdf,
}

impl TpmSSchemeXor {
    pub fn new(hash_alg: TpmIAlgHash, kdf: TpmIAlgKdf) -> Self {
        Self {
            hash_alg: hash_alg,
            kdf: kdf,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.hash_alg.size() + self.kdf.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.hash_alg.to_vec());
        buf.extend_from_slice(&self.kdf.to_vec());
        buf
    }
}

// Table 3:152 - TPMS_SCHEME_ECDAA
pub struct TpmSSchemeEcdaa {
    pub hash_alg: TpmIAlgHash,
    pub count: u16,
}

impl TpmSSchemeEcdaa {
    pub fn new(hash_alg: TpmIAlgHash, count: u16) -> Self {
        Self {
            hash_alg: hash_alg,
            count: count,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.hash_alg.size() + mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.hash_alg.to_vec());
        buf.extend_from_slice(&u16::to_be_bytes(self.count));
        buf
    }
}

// Table 3:156 - TPMU_SCHEME_KEYEDHASH
pub struct TpmUSchemeKeyedHash {
    pub selector: TpmAlgorithms,
    pub scheme_hmac: Option<TpmSSchemeHmac>,
    pub scheme_xor: Option<TpmSSchemeXor>,
}

impl TpmUSchemeKeyedHash {
    pub fn new(selector: TpmAlgorithms,
               scheme_hmac: Option<TpmSSchemeHmac>,
               scheme_xor: Option<TpmSSchemeXor>) -> Self {
        match selector {
            TpmAlgorithms::TPM_ALG_HMAC |
            TpmAlgorithms::TPM_ALG_XOR  |
            TpmAlgorithms::TPM_ALG_NULL  =>
                Self {
                    selector: selector,
                    scheme_hmac: scheme_hmac,
                    scheme_xor: scheme_xor,
                },
            _ =>
                Self {
                    selector: TpmAlgorithms::TPM_ALG_NULL,
                    scheme_hmac: None,
                    scheme_xor: None,
                },
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = 0;
        if let TpmAlgorithms::TPM_ALG_HMAC = self.selector {
            ret = self.scheme_hmac.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_XOR = self.selector {
            ret = self.scheme_xor.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_NULL = self.selector {
            ret = 0;
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        if let TpmAlgorithms::TPM_ALG_HMAC = self.selector {
            buf.extend_from_slice(&self.scheme_hmac.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_XOR = self.selector {
            buf.extend_from_slice(&self.scheme_xor.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_NULL = self.selector {
            ();
        }
        buf
    }
}

// Table 3:157 - TPMT_KEYEDHASH_SCHEME
pub struct TpmTKeyedhashScheme {
    pub scheme: u16, // TPM_ALG_HMAC, TPM_ALG_XOR, or TPM_ALG_NULL
    pub details: TpmUSchemeKeyedHash,
}

impl TpmTKeyedhashScheme {
    pub fn new(scheme: u16, details: TpmUSchemeKeyedHash) -> Self {
        Self {
            scheme: scheme,
            details: details,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + self.details.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.scheme));
        buf.extend_from_slice(&self.details.to_vec());
        buf
    }
}

// Table 3:194 - TPMT_KEYEDHASH_SCHEME
pub struct TpmSKeyedhashParms {
    pub scheme: TpmTKeyedhashScheme,
}

impl TpmSKeyedhashParms {
    pub fn new(scheme: TpmTKeyedhashScheme) -> Self {
        Self {
            scheme: scheme,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.scheme.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.scheme.to_vec());
        buf
    }
}

// Table 3:71 - TPMI_ALG_SIG_SCHEME
pub struct TpmIAlgSigScheme {
    pub sig_scheme: u16,
}

impl TpmIAlgSigScheme {
    pub fn new(sig_scheme: u16) -> Self {
        Self {
            sig_scheme: sig_scheme,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.sig_scheme));
        buf
    }
}

// Table 3:160 - TPMU_SIG_SCHEME
pub struct TpmUSigScheme {
    pub selector: TpmAlgorithms,
    pub scheme_hmac: Option<TpmSSchemeHmac>,
    pub scheme_hash: Option<TpmSSchemeHash>,
    pub scheme_ecdaa: Option<TpmSSchemeEcdaa>,
}

impl TpmUSigScheme {
    pub fn new(selector: TpmAlgorithms,
               scheme_hmac: Option<TpmSSchemeHmac>,
               scheme_hash: Option<TpmSSchemeHash>,
               scheme_ecdaa: Option<TpmSSchemeEcdaa>) -> Self {
        match selector {
            TpmAlgorithms::TPM_ALG_HMAC  |
            TpmAlgorithms::TPM_ALG_RSA   |
            TpmAlgorithms::TPM_ALG_ECDAA |
            TpmAlgorithms::TPM_ALG_NULL  =>
                Self {
                    selector: selector,
                    scheme_hmac: scheme_hmac,
                    scheme_hash: scheme_hash,
                    scheme_ecdaa: scheme_ecdaa,
                },
            _ =>
                Self {
                    selector: TpmAlgorithms::TPM_ALG_NULL,
                    scheme_hmac: None,
                    scheme_hash: None,
                    scheme_ecdaa: None,
                },
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = 0;
        if let TpmAlgorithms::TPM_ALG_HMAC = self.selector {
            ret = self.scheme_hmac.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_RSA = self.selector {
            ret = self.scheme_hash.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_ECDAA = self.selector {
            ret = self.scheme_ecdaa.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_NULL = self.selector {
            ret = 0;
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        if let TpmAlgorithms::TPM_ALG_HMAC = self.selector {
            buf.extend_from_slice(&self.scheme_hmac.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_RSA = self.selector {
            buf.extend_from_slice(&self.scheme_hash.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_ECDAA = self.selector {
            buf.extend_from_slice(&self.scheme_ecdaa.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_NULL = self.selector {
            ();
        }
        buf
    }
}

// Table 3:161 - TPMT_SIG_SCHEME
pub struct TpmTSigScheme {
    pub scheme: TpmIAlgSigScheme,
    pub details: TpmUSigScheme,
}

impl TpmTSigScheme {
    pub fn new(scheme: TpmIAlgSigScheme, details: TpmUSigScheme) -> Self {
        Self {
            scheme: scheme,
            details: details,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.scheme.size() + self.details.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.scheme.to_vec());
        buf.extend_from_slice(&self.details.to_vec());
        buf
    }
}

// Table 3:70 - TPMI_ALG_KDF
pub struct TpmIAlgKdf {
    pub kdf: u16,
}

impl TpmIAlgKdf {
    pub fn new(kdf: u16) -> Self {
        Self {
            kdf: kdf,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.kdf));
        buf
    }
}

// Table 3:65 - TPMI_ALG_HASH
pub struct TpmIAlgHash {
    pub hash: u16,
}

impl TpmIAlgHash {
    pub fn new(hash: u16) -> Self {
        Self {
            hash: hash,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.hash));
        buf
    }
}

// Table 3:67 - TPMI_ALG_SYM
pub struct TpmIAlgSym {
    pub sym: u16,
}

impl TpmIAlgSym {
    pub fn new(sym: u16) -> Self {
        Self {
            sym: sym,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.sym));
        buf
    }
}

// Table 3:68 - TPMI_ALG_SYM_OBJECT
pub struct TpmIAlgSymObject {
    pub sym_obj: u16, // Only TPM_ALG_AES is currently supported
}

impl TpmIAlgSymObject {
    pub fn new(sym_obj: u16) -> Self {
        Self {
            sym_obj: sym_obj,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.sym_obj));
        buf
    }
}

// Table 3:xx - TPMI_AES_KEY_BITS (4.12.5)
pub struct TpmIAesKeyBits {
    pub aes_key_sizes_bits: u16, // Only 128 or 256 is supported
}

impl TpmIAesKeyBits {
    pub fn new(aes_key_sizes_bits: u16) -> Self {
        Self {
            aes_key_sizes_bits: aes_key_sizes_bits,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.aes_key_sizes_bits));
        buf
    }
}

// Table 3:137 - TPMU_SYM_KEY_BITS
pub struct TpmUSymKeyBits {
    pub aes_key_bits: TpmIAesKeyBits, // Only AES is supported
}

impl TpmUSymKeyBits {
    pub fn new(aes_key_bits: TpmIAesKeyBits) -> Self {
        Self {
            aes_key_bits: aes_key_bits,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.aes_key_bits.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.aes_key_bits.to_vec());
        buf
    }
}

// Table 3:69 - TPMI_ALG_SYM_MODE
pub struct TpmIAlgSymMode {
    pub mode: u16,
}

impl TpmIAlgSymMode {
    pub fn new(mode: u16) -> Self {
        Self {
            mode: mode,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.mode));
        buf
    }
}

// Table 3:138 - TPMU_SYM_MODE
// TODO: Support other algorithms
pub struct TpmUSymMode {
    pub aes: TpmIAlgSymMode, // Only AES is supported
}

impl TpmUSymMode{
    pub fn new(aes: TpmIAlgSymMode) -> Self {
        Self {
            aes: aes,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.aes.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.aes.to_vec());
        buf
    }
}

// Table 3:140 - TPMT_SYM_DEF
pub struct TpmTSymDef {
    pub algorithm: TpmIAlgSym,
    pub key_bits: Option<TpmUSymKeyBits>,
    pub mode: Option<TpmUSymMode>,
}

impl TpmTSymDef {
    pub fn new(algorithm: TpmIAlgSym, key_bits: Option<TpmUSymKeyBits>,
               mode: Option<TpmUSymMode>) -> Self {
        Self {
            algorithm: algorithm,
            key_bits: key_bits,
            mode: mode,
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = self.algorithm.size();
        match &self.key_bits {
            Some(x) => ret += x.size(),
            None => (),
        }
        match &self.mode {
            Some(x) => ret += x.size(),
            None => (),
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.algorithm.to_vec());
        match &self.key_bits {
            Some(x) => buf.extend_from_slice(&x.to_vec()),
            None => (),
        }
        match &self.mode {
            Some(x) => buf.extend_from_slice(&x.to_vec()),
            None => (),
        }
        buf
    }
}

// Table 3:141 - TPMT_SYM_DEF_OBJECT
pub struct TpmTSymDefObject {
    pub algorithm: TpmIAlgSymObject,
    pub key_bits: Option<TpmUSymKeyBits>,
    pub mode: Option<TpmUSymMode>,
}

impl TpmTSymDefObject {
    pub fn new(algorithm: TpmIAlgSymObject, key_bits: Option<TpmUSymKeyBits>,
               mode: Option<TpmUSymMode>) -> Self {
        Self {
            algorithm: algorithm,
            key_bits: key_bits,
            mode: mode,
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = self.algorithm.size();
        match &self.key_bits {
            Some(x) => ret += x.size(),
            None => (),
        }
        match &self.mode {
            Some(x) => ret += x.size(),
            None => (),
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.algorithm.to_vec());
        match &self.key_bits {
            Some(x) => buf.extend_from_slice(&x.to_vec()),
            None => (),
        }
        match &self.mode {
            Some(x) => buf.extend_from_slice(&x.to_vec()),
            None => (),
        }
        buf
    }
}

// Table 3:171 - TPMT_RSA_SCHEME
pub struct TpmTRsaScheme {
    pub scheme: u16,
    pub details: Option<u16>,
}

impl TpmTRsaScheme {
    pub fn new(scheme: u16, details: Option<u16>) -> Self {
        Self {
            scheme: scheme,
            details: details,
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = mem::size_of::<u16>();
        match &self.details {
            Some(x) => ret += mem::size_of::<u16>(),
            None => (),
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.scheme));
        match &self.details {
            Some(x) => buf.extend_from_slice(&u16::to_be_bytes(*x)),
            None => (),
        }
        buf
    }
}

// Table 3:196 - TPMS_RSA_PARMS
pub struct TpmSRsaParms {
    pub symmetric: TpmTSymDefObject,
    pub scheme: TpmTRsaScheme,
    pub key_bits: u16,
    pub exponent: u32,
}

impl TpmSRsaParms {
    pub fn new(symmetric: TpmTSymDefObject, scheme: TpmTRsaScheme,
               key_bits: u16, exponent: u32) -> Self {
        Self {
            symmetric: symmetric,
            scheme: scheme,
            key_bits: key_bits,
            exponent: exponent,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize =   self.symmetric.size()
                         + self.scheme.size()
                         + mem::size_of::<u16>()
                         + mem::size_of::<u32>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.symmetric.to_vec());
        buf.extend_from_slice(&self.scheme.to_vec());
        buf.extend_from_slice(&u16::to_be_bytes(self.key_bits));
        buf.extend_from_slice(&u32::to_be_bytes(self.exponent));
        buf
    }
}

// Table 3:143 - TPMS_SYMCIPHER_PARMS
pub struct TpmSSymcipherParms {
    pub sym: TpmTSymDefObject,
}

impl TpmSSymcipherParms {
    pub fn new(sym: TpmTSymDefObject) -> Self {
        Self {
            sym: sym,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = self.sym.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&self.sym.to_vec());
        buf
    }
}

// Table 3:198 - TPMU_PUBLIC_PARMS
pub struct TpmUPublicParms {
    pub selector: TpmAlgorithms,
    pub keyedhash_parms: Option<TpmSKeyedhashParms>,
    pub symcipher_parms: Option<TpmSSymcipherParms>,
    pub rsa_parms: Option<TpmSRsaParms>,
    // ToDo: Add TPMS_ECC_PARMS and TPMS_ASYM_PARMS support
}

impl TpmUPublicParms {
    pub fn new(selector: TpmAlgorithms,
               keyedhash_parms: Option<TpmSKeyedhashParms>,
               symcipher_parms: Option<TpmSSymcipherParms>,
               rsa_parms: Option<TpmSRsaParms>) -> Self {
        match selector {
            TpmAlgorithms::TPM_ALG_KEYEDHASH |
            TpmAlgorithms::TPM_ALG_SYMCIPHER |
            TpmAlgorithms::TPM_ALG_RSA =>
                Self {
                    selector: selector,
                    keyedhash_parms: keyedhash_parms,
                    symcipher_parms: symcipher_parms,
                    rsa_parms: rsa_parms,
                },
            _ =>
                Self {
                    selector: TpmAlgorithms::TPM_ALG_NULL,
                    keyedhash_parms: None,
                    symcipher_parms: None,
                    rsa_parms: None,
                },
        }
    }

    pub fn size(&self) -> usize {
        let mut ret: usize = 0;
        if let TpmAlgorithms::TPM_ALG_KEYEDHASH = self.selector {
            ret = self.keyedhash_parms.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_SYMCIPHER = self.selector {
            ret = self.symcipher_parms.as_ref().unwrap().size();
        }
        if let TpmAlgorithms::TPM_ALG_RSA = self.selector {
            ret = self.rsa_parms.as_ref().unwrap().size();
        }
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        if let TpmAlgorithms::TPM_ALG_KEYEDHASH = self.selector {
            buf.extend_from_slice(&self.keyedhash_parms.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_SYMCIPHER = self.selector {
            buf.extend_from_slice(&self.symcipher_parms.as_ref().unwrap().to_vec());
        }
        if let TpmAlgorithms::TPM_ALG_RSA = self.selector {
            buf.extend_from_slice(&self.rsa_parms.as_ref().unwrap().to_vec());
        }
        buf
    }
}

// Table 3:174 - TPM2B_PUBLIC_KEY_RSA
pub struct Tpm2BPublicKeyRsa {
    pub size: u16,
    pub buffer: Vec<u8>,
}

impl Tpm2BPublicKeyRsa {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            size: buffer.len() as u16,
            buffer: buffer,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize =   mem::size_of::<u16>()
                         + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.buffer);
        }
        buf
    }
}

// Table 3:200 - TPMT_PUBLIC
pub struct TpmTPublic {
    pub alg_type: u16,
    pub name_alg: u16,
    pub object_attributes: u32,
    pub auth_policy: Tpm2BDigest,
    pub parameters: TpmUPublicParms,
    pub unique: Tpm2BPublicKeyRsa,
}

impl TpmTPublic {
    pub fn new(alg_type: u16, name_alg: u16, object_attributes: u32,
               auth_policy: Tpm2BDigest, parameters: TpmUPublicParms,
               unique: Tpm2BPublicKeyRsa) -> Self {
        Self {
            alg_type: alg_type,
            name_alg: name_alg,
            object_attributes: object_attributes,
            auth_policy: auth_policy,
            parameters: parameters,
            unique: unique,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize =   mem::size_of::<u16>() * 2
                         + mem::size_of::<u32>()
                         + self.auth_policy.size()
                         + self.parameters.size()
                         + self.unique.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.alg_type));
        buf.extend_from_slice(&u16::to_be_bytes(self.name_alg));
        buf.extend_from_slice(&u32::to_be_bytes(self.object_attributes));
        buf.extend_from_slice(&self.auth_policy.to_vec());
        buf.extend_from_slice(&self.parameters.to_vec());
        buf.extend_from_slice(&self.unique.to_vec());
        buf
    }
}

// Table 3:201 - TPM2B_PUBLIC
pub struct Tpm2BPublic {
    pub size: u16,
    pub public_area: TpmTPublic,
}

impl Tpm2BPublic {
    pub fn new(public_area: TpmTPublic) -> Self {
        Self {
            size: public_area.size() as u16,
            public_area: public_area,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize =   mem::size_of::<u16>()
                         + self.public_area.size();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.public_area.to_vec());
        }
        buf
    }
}

// Table 3:41 - TPMI_DH_OBJECT
pub struct TpmIDhObject {
    pub object: u32,
}

impl TpmIDhObject {
    pub fn new(object: u32) -> Self {
        Self {
            object: object,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u32>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(self.object));
        buf
    }
}

// Table 3:44 - TPMI_DH_ENTITY
pub struct TpmIDhEntity {
    pub entity: u32,
}

impl TpmIDhEntity {
    pub fn new(entity: u32) -> Self {
        Self {
            entity: entity,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u32>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(self.entity));
        buf
    }
}

// Table 3:82 - TPM2B_NONCE
pub struct Tpm2BNonce {
    pub size: u16,
    pub nonce: Vec<u8>,
}

impl Tpm2BNonce {
    pub fn new(nonce: Vec<u8>) -> Self {
        Self {
            size: nonce.len() as u16,
            nonce: nonce,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.nonce);
        }
        buf
    }
}

// Table 3:191 - TPM2B_ENCRYPTED_SECRET
pub struct Tpm2BEncryptedSecret {
    pub size: u16,
    pub secret: Vec<u8>,
}

impl Tpm2BEncryptedSecret {
    pub fn new(secret: Vec<u8>) -> Self {
        Self {
            size: secret.len() as u16,
            secret: secret,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + (self.size as usize) * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.size));
        if self.size > 0 {
            buf.extend_from_slice(&self.secret);
        }
        buf
    }
}

// Table 3:48 - TPMI_SH_POLICY
pub struct TpmIShPolicy {
    pub policy: u32,
}

impl TpmIShPolicy {
    pub fn new(policy: u32) -> Self {
        Self {
            policy: policy,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u32>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u32::to_be_bytes(self.policy));
        buf
    }
}
