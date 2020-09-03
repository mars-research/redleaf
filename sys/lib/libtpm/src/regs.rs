// Register definitions of all TPM registers
// Derived from https://trustedcomputinggroup.org/wp-content/uploads/TCG_PCClientTPMInterfaceSpecification_TIS__1-3_27_03212013.pdf
//
//
use alloc::vec::Vec;
use byteorder::{ByteOrder, BigEndian};

pub const TPM_HEADER_SIZE: usize = 10;
pub const TPM_PLATRFORM_PCR: usize = 24;
pub const TPM_PCR_SELECT_MIN: usize = (TPM_PLATRFORM_PCR + 7) / 8;

bitfield! {
    pub struct TpmAccess(u8);
    impl Debug;
    pub tpm_reg_validsts, _: 7;
    pub active_locality, set_active_locality: 5;
    pub been_seized, set_been_seized: 4;
    pub seize, set_seize: 3;
    pub pending_request, _: 2;
    pub request_use, set_request_use: 1;
    pub tpm_establishment, _: 0;
}

bitfield! {
    pub struct TpmStatus(u8);
    impl Debug;
    u8;
    pub sts_valid, set_sts_valid: 7;
    pub command_ready, set_command_ready: 6;
    pub tpm_go, set_tpm_go: 5;
    pub data_avail, set_data_avail: 4;
    pub expect, _: 3;
    pub selftest_done, _: 2;
    pub response_retry, set_response_retry: 1;
    pub rsvd, _: 0;
}

bitfield! {
    pub struct TpmIntfCap(u32);
    impl Debug;
    u32;
    rsvd_31, _: 31;
    pub iface_ver, _: 30, 28;
    rsvd_27, _: 27, 11;
    pub data_xfer_size, _: 10, 9;
    pub burst_count_static, _: 8;
    pub cmd_ready_int, _: 7;
    pub irq_edge_falling, _: 6;
    pub irq_edge_rising, _: 5;
    pub irq_level_low, _: 4;
    pub irq_level_high, _: 3;
    pub locality_change_int, _: 2;
    pub sts_valid_int, _: 1;
    pub data_avail_int, _: 0;
}

bitfield! {
    pub struct TpmIntEnable(u32);
    impl Debug;
    u32;
    pub global_int_enable, set_global_int_enable: 31;
    rsvd, _: 30, 8;
    pub cmd_ready_enable, set_command_ready_enable: 7;
    rsvd2, _: 6, 5;
    pub type_polarity, set_type_polarity: 4, 3;
    pub locality_change_int, _: 2;
    pub sts_valid_int_enable, set_sts_valid_int_enable: 1;
    pub data_avail_int_enable, set_data_avail_int_enable: 0;
}

bitfield! {
    pub struct TpmIntStatus(u8);
    impl Debug;
    u8;
    pub cmd_ready_int, set_cmd_ready_int_occ: 7;
    rsvd, _: 6, 3;
    pub locality_change_int_occ, set_locality_change_int_occ: 2;
    pub sts_valid_int_occ, set_sts_valid_int_occ: 1;
    pub data_avail_int_occ, set_data_avail_int_occ: 0;
}

bitfield! {
    pub struct TpmIntVector(u8);
    impl Debug;
    u8;
    pub sirqvec, _: 3, 0;
}

bitfield! {
    pub struct TpmAObject(u32);
    impl Debug;
    u32;
    pub rsvd_31, _: 31, 20;
    pub x509sign, set_x509sign: 19;
    pub sign, set_sign: 18;
    pub decrypt, set_decrypt: 17;
    pub restricted, set_restricted: 16;
    pub rsvd_15, _: 15, 12;
    pub encrypted_duplication, set_encrypted_duplication: 11;
    pub no_da, set_no_da: 10;
    pub rsvd_9, _: 9, 8;
    pub admin_with_policy, set_admin_with_policy: 7;
    pub user_with_auth, set_user_with_auth: 6;
    pub sensitive_data_origin, set_sensitive_data_origin: 5;
    pub fixed_parent, set_fixed_parent: 4;
    pub rsvd_3, _: 3;
    pub st_clear, set_st_clear: 2;
    pub fixed_tpm, set_fixed_tpm: 1;
    pub rsvd, _: 0;
}

// Generously borrowed from linux/drivers/char/tpm/tpm.h
pub enum Tpm2Commands {
    TPM2_CC_FIRST		        = 0x011F,
    TPM2_CC_HIERARCHY_CONTROL       = 0x0121,
    TPM2_CC_HIERARCHY_CHANGE_AUTH   = 0x0129,
    TPM2_CC_CREATE_PRIMARY          = 0x0131,
    TPM2_CC_SEQUENCE_COMPLETE       = 0x013E,
    TPM2_CC_SELF_TEST	        = 0x0143,
    TPM2_CC_STARTUP		        = 0x0144,
    TPM2_CC_SHUTDOWN	        = 0x0145,
    TPM2_CC_NV_READ                 = 0x014E,
    TPM2_CC_CREATE		        = 0x0153,
    TPM2_CC_LOAD		        = 0x0157,
    TPM2_CC_SEQUENCE_UPDATE         = 0x015C,
    TPM2_CC_UNSEAL		        = 0x015E,
    TPM2_CC_CONTEXT_LOAD	        = 0x0161,
    TPM2_CC_CONTEXT_SAVE	        = 0x0162,
    TPM2_CC_FLUSH_CONTEXT	        = 0x0165,
    TPM2_CC_POLICY_LOCALITY         = 0x016F,
    TPM2_CC_START_AUTH_SESSION        = 0x0176,
    TPM2_CC_VERIFY_SIGNATURE        = 0x0177,
    TPM2_CC_GET_CAPABILITY	        = 0x017A,
    TPM2_CC_GET_RANDOM	        = 0x017B,
    TPM2_CC_HASH	        = 0x017D,
    TPM2_CC_PCR_READ	        = 0x017E,
    TPM2_CC_POLICY_PCR         = 0x017F,
    TPM2_CC_PCR_EXTEND	        = 0x0182,
    TPM2_CC_EVENT_SEQUENCE_COMPLETE = 0x0185,
    TPM2_CC_HASH_SEQUENCE_START     = 0x0186,
    TPM2_CC_POLICY_GET_DIGEST     = 0x0189,
    TPM2_CC_CREATE_LOADED           = 0x0191,
    TPM2_CC_LAST		        = 0x0193,
}

// Generously borrowed from linux/drivers/char/tpm/tpm.h
pub enum Tpm2ReturnCodes {
    TPM2_RC_SUCCESS		= 0x0000,
    TPM2_RC_HASH		= 0x0083, /* RC_FMT1 */
    TPM2_RC_HANDLE		= 0x008B,
    TPM2_RC_INITIALIZE	= 0x0100, /* RC_VER1 */
    TPM2_RC_FAILURE		= 0x0101,
    TPM2_RC_DISABLED	= 0x0120,
    TPM2_RC_COMMAND_CODE    = 0x0143,
    TPM2_RC_TESTING		= 0x090A, /* RC_WARN */
    TPM2_RC_REFERENCE_H0	= 0x0910,
    TPM2_RC_RETRY		= 0x0922,
    TPM2_RC_NOT_USED		= 0x097F,
}

// Generously borrowed from linux/drivers/char/tpm/tpm.h
pub enum TpmStructures {
    TPM_ST_RSP_COMMAND           = 0x00C4,
    TPM_ST_NULL                  = 0x8000,
    TPM_ST_NO_SESSIONS           = 0x8001,
    TPM_ST_SESSIONS	             = 0x8002,
    TPM_ST_ATTEST_NV             = 0x8014,
    TPM_ST_ATTEST_COMMAND_AUDIT  = 0x8015,
    TPM_ST_ATTEST_SESSION_AUDIT  = 0x8016,
    TPM_ST_ATTEST_CERTIFY        = 0x8017,
    TPM_ST_ATTEST_QUOTE          = 0x8018,
    TPM_ST_ATTEST_TIME           = 0x8019,
    TPM_ST_ATTEST_CREATION       = 0x801A,
    TPM_ST_ATTEST_NV_DIGEST      = 0x801C,
    TPM_ST_CREATION              = 0x8021,
    TPM_ST_VERIFIED              = 0x8022,
    TPM_ST_AUTH_SECRET           = 0x8023,
    TPM_ST_HASHCHECK             = 0x8024,
    TPM_ST_AUTH_SIGNED           = 0x8025,
    TPM_ST_FU_MANIFEST           = 0x8029,
}

pub enum TpmRH {
    TPM_RH_FIRST       = 0x40000000,
    TPM_RH_OWNER       = 0x40000001,
    TPM_RH_NULL        = 0x40000007,
    TPM_RH_UNASSIGNED  = 0x40000008,
    TPM_RS_PW          = 0x40000009,
    TPM_RS_LOCKOUT     = 0x4000000A,
    TPM_RS_ENDORSEMENT = 0x4000000B,
    TPM_RS_PLATFORM    = 0x4000000C,
    TPM_RS_PLATFORM_NV = 0x4000000D,
    TPM_RS_AUTH_00     = 0x40000010,
    TPM_RS_AUTH_FF     = 0x4000010F,
    TPM_RS_ACT_0       = 0x40000110,
    TPM_RS_ACT_F       = 0x4000011F,
    // TPM_RS_LAST        = 0x4000011F,
}

pub enum TpmSE {
    TPM_SE_HMAC   = 0x00,
    TPM_SE_POLICY = 0x01,
    TPM_SE_TRIAL  = 0x03,
}

pub const TIMEOUT_A:       usize = 750;
pub const TIMEOUT_B:       usize = 2000;
pub const TIMEOUT_C:       usize = 200;
pub const TIMEOUT_D:       usize = 30;
pub const DURATION_SHORT:  usize = 20;
pub const DURATION_MEDIUM: usize = 750;
pub const DURATION_LONG:   usize = 2000;

// Generously borrowed from linux/drivers/char/tpm/tpm.h
#[derive(Copy, Clone)]
pub enum TpmAlgorithms {
    TPM_ALG_ERROR		= 0x0000,
    TPM_ALG_RSA		    = 0x0001,
    TPM_ALG_SHA1		= 0x0004,
    TPM_ALG_HMAC		= 0x0005,
    TPM_ALG_AES		    = 0x0006,
    TPM_ALG_KEYEDHASH	= 0x0008,
    TPM_ALG_XOR		    = 0x000A,
    TPM_ALG_SHA256		= 0x000B,
    TPM_ALG_SHA384		= 0x000C,
    TPM_ALG_SHA512		= 0x000D,
    TPM_ALG_NULL		= 0x0010,
    TPM_ALG_SM3_256		= 0x0012,
    TPM_ALG_ECC		    = 0x0023,
    TPM_ALG_SYMCIPHER   = 0x0025,
    TPM_ALG_CTR		    = 0x0040,
    TPM_ALG_OFB		    = 0x0041,
    TPM_ALG_CBC		    = 0x0042,
    TPM_ALG_CFB		    = 0x0043,
    TPM_ALG_ECB		    = 0x0044,
}

// Generously borrowed from include/uapi/linux/hash_info.h
pub enum HashAlgorithms {
	HASH_ALGO_MD4 = 0,
	HASH_ALGO_MD5,
	HASH_ALGO_SHA1,
	HASH_ALGO_RIPE_MD_160,
	HASH_ALGO_SHA256,
	HASH_ALGO_SHA384,
	HASH_ALGO_SHA512,
	HASH_ALGO_SHA224,
	HASH_ALGO_RIPE_MD_128,
	HASH_ALGO_RIPE_MD_256,
	HASH_ALGO_RIPE_MD_320,
	HASH_ALGO_WP_256,
	HASH_ALGO_WP_384,
	HASH_ALGO_WP_512,
	HASH_ALGO_TGR_128,
	HASH_ALGO_TGR_160,
	HASH_ALGO_TGR_192,
	HASH_ALGO_SM3_256,
	HASH_ALGO_STREEBOG_256,
	HASH_ALGO_STREEBOG_512,
	HASH_ALGO__LAST
}

// Generously borrowed from linux/drivers/char/tpm/tpm.h
pub enum Tpm2Capabilities {
    TPM2_CAP_HANDLES	= 1,
    TPM2_CAP_COMMANDS	= 2,
    TPM2_CAP_PCRS		= 5,
    TPM2_CAP_TPM_PROPERTIES = 6,
}

#[repr(packed)]
pub struct TpmBankInfo {
    pub alg_id: u16,
    pub digest_size: u16,
    pub crypto_id: u16,
}

impl TpmBankInfo {
    pub fn new(alg_id: u16, digest_size: u16, crypto_id: u16) -> Self {
        Self {
            alg_id:      alg_id.swap_bytes().to_be(),
            digest_size: digest_size.swap_bytes().to_be(),
            crypto_id:   crypto_id.swap_bytes().to_be(),
        }
    }
}

#[repr(packed)]
pub struct TpmHeader {
	pub tag: u16,
	pub length: u32,
	pub ordinal: u32,
}

impl TpmHeader {
    pub fn from_vec(buf: &Vec <u8>) -> TpmHeader {
        let slice = buf.as_slice();
        TpmHeader {
            tag:     BigEndian::read_u16(&slice[0..2]),
            length:  BigEndian::read_u32(&slice[2..6]),
            ordinal: BigEndian::read_u32(&slice[6..10]),
        }
    }

    pub fn to_vec(hdr: &TpmHeader) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(10);
        buf.extend_from_slice(&u16::to_be_bytes(hdr.tag));
        buf.extend_from_slice(&u32::to_be_bytes(hdr.length));
        buf.extend_from_slice(&u32::to_be_bytes(hdr.ordinal));
        buf
    }

    pub fn new(tag: u16, length: u32, ordinal: u32) -> Self {
        Self {
            tag:     tag.swap_bytes().to_be(),
            length:  length.swap_bytes().to_be(),
            ordinal: ordinal.swap_bytes().to_be(),
        }
    }
}

pub struct TpmDevInfo {
    pub nr_allocated_banks: u32,
    pub allocated_banks: Vec<TpmBankInfo>,
}

impl TpmDevInfo {
    pub fn new(nr_allocated_banks: u32, allocated_banks: Vec<TpmBankInfo>) -> Self {
        Self {
            nr_allocated_banks: nr_allocated_banks,
            allocated_banks: allocated_banks,
        }
    }
}
