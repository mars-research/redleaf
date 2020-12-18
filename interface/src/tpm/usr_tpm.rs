use alloc::vec::Vec;
use alloc::boxed::Box;
use core::mem;

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

// Table 3:79 - TPMT_HA
pub struct TpmTHa {
    pub hash_alg: u16,
    pub digest: Vec<u8>,
}

impl TpmTHa {
    pub fn new(hash_alg: u16, digest: Vec<u8>) -> Self {
        Self {
            hash_alg: hash_alg,
            digest: digest,
        }
    }

    pub fn size(&self) -> usize {
        let ret: usize = mem::size_of::<u16>() + self.digest.len() * mem::size_of::<u8>();
        ret
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size());
        buf.extend_from_slice(&u16::to_be_bytes(self.hash_alg));
        buf.extend_from_slice(&self.digest);
        buf
    }
}

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
    TPM_ALG_RSASSA		= 0x0014,
    TPM_ALG_ECDAA		= 0x001A,
    TPM_ALG_ECC		    = 0x0023,
    TPM_ALG_SYMCIPHER   = 0x0025,
    TPM_ALG_CTR		    = 0x0040,
    TPM_ALG_OFB		    = 0x0041,
    TPM_ALG_CBC		    = 0x0042,
    TPM_ALG_CFB		    = 0x0043,
    TPM_ALG_ECB		    = 0x0044,
}

pub enum TpmSE {
    TPM_SE_HMAC   = 0x00,
    TPM_SE_POLICY = 0x01,
    TPM_SE_TRIAL  = 0x03,
}

// Rv6 user -[UsrTpm]-> driver -[TpmDev]-> TPM
// #[interface]
pub trait UsrTpm: Send + Sync {
    /// Create a clone of the TPM interface that points to the same driver.
    fn clone_usrtpm(&self) -> Box<dyn UsrTpm>;

    /// ## Locality related functions
    ///
    /// Locality tells the TPM where the command originated.
    /// Validates the TPM locality, basically means that TPM is ready to listen for commands and
    /// perform operation in this locality.
    /// Ref: https://ebrary.net/24811/computer_science/locality_command
    fn tpm_validate_locality(&self, locality: u32) -> bool;

    /// Explicitly giveup locality. This may not be useful if there is only a single process/user using
    /// TPM in an OS. In multi-user scenario, this is more applicable.
    fn relinquish_locality(&self, locality: u32) -> bool;

    fn tpm_deactivate_all_localities(&self) -> bool;

    /// Requests the TPM to switch to the locality we choose and wait for TPM to acknowledge our
    /// request
    fn tpm_request_locality(&self, locality: u32) -> bool;

    /// Reads the TPM ID from device register
    fn read_tpm_id(&self, locality: u32);

    /// Reads the burst_count from TPM register. Burst count is the amount of bytes the TPM device is
    /// capable of handling in oneshot.
    fn tpm_get_burst(&self, locality: u32) -> u16;

    /// Busy-wait in a loop for a particular status flag to be set
    fn wait_for_status_flag(&self, locality: u32, flag: u8, timeout_ms: usize) -> bool;

    /// Writes data to the TPM FIFO.
    /// Here, `data.len < burst_count`
    fn tpm_write_data(&self, locality: u32, data: &[u8]) -> usize;

    /// Checks TPM status register to see if there is any data available
    fn is_data_available(&self, locality: u32) -> bool;

    /// Read data from TPM
    /// * Wait for data to be available
    /// * Receive as much as burst_count
    fn tpm_read_data(&self, locality: u32, data: &mut [u8]) -> usize;

    /// Wrapper for `tpm_read_data`
    /// This function first tries to read TPM_HEADER_SIZE bytes from the TPM to determine the length of
    /// payload data.
    /// Then it issues a second read for the length of payload data subtract TPM_HEADER_SIZE
    /// Payload consists of the argument that was sent to the TPM during tpm_send_data and the response
    fn tpm_recv_data(&self, locality: u32, buf: &mut Vec<u8>, rc: &mut u32) -> usize;

    /// Wrapper for `tpm_write_data`
    /// This function waits for TPM to be in a state to accept commands before writing data to FIFO.
    fn tpm_send_data(&self, locality: u32, buf: &mut Vec<u8>) -> usize;

    /// Transmit command to a TPM.
    /// This function does a bi-directional communication with TPM.
    /// First, it sends a command with headers
    /// If successful, try to read the response buffer from TPM
    fn tpm_transmit_cmd(&self, locality: u32, buf: &mut Vec<u8>);

    /// Table 3:68 - TPM2_GetRandom Command
    /// Get a random number from TPM.
    /// `num_octets` represents the length of the random number in bytes
    fn tpm_get_random(&self, locality: u32, num_octets: usize) -> bool;

    /// Table 3:114 - TPM2_PCR_Read Command
    /// Read a PCR register.
    /// Since the communication channel between the process and the TPM is untrusted,
    /// TPM2_Quote should be the command to retreive PCR values, not TPM2_PCR_Read
    fn tpm_pcr_read(
        &self,
        locality: u32,
        pcr_idx: usize,
        hash: u16,
        digest_size: &mut u16,
        digest: &mut Vec<u8>,
    ) -> bool;

    /// Obtain information about banks that are allocated in TPM
    fn tpm_init_bank_info(&self, locality: u32, hash_alg: u16) -> TpmBankInfo;

    /// Table 3:208 - TPM2_PCR_GetCapability Command
    /// Obtain the banks that are allocated in TPM
    /// TODO: Return true/false, not structure
    fn tpm_get_pcr_allocation(&self, locality: u32) -> TpmDevInfo;

    /// Table 3:110 - TPM2_PCR_Read Command
    /// Extend PCR register.
    /// The value sent to the TPM will be concatenated with the original value and hashed.
    fn tpm_pcr_extend(
        &self,
        locality: u32,
        tpm_info: &TpmDevInfo,
        pcr_idx: usize,
        digest_values: Vec<TpmTHa>,
    ) -> bool;

    /// Table 3:78 - TPM2_HashSequenceStart Command
    /// Conduct hash calculation in TPM
    fn tpm_hash_sequence_start(&self, locality: u32, hash: TpmAlgorithms, object: &mut u32) -> bool;

    /// Table 3:80 - TPM2_SequenceUpdate
    /// Update hash calculation in TPM
    fn tpm_sequence_update(&self, locality: u32, object: u32, buffer: Vec<u8>) -> bool;

    /// Table 3:82 - TPM2_SequenceComplete
    /// Finalize hash calculation in TPM
    fn tpm_sequence_complete(
        &self,
        locality: u32,
        object: u32,
        buffer: Vec<u8>,
        hash_size: &mut u16,
        hash: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:62 - TPM2_Hash
    /// Generic hash calculation in TPM when data size is known
    fn tpm_hash(
        &self,
        locality: u32,
        hash: TpmAlgorithms,
        buffer: Vec<u8>,
        hash_size: &mut u16,
        hash_val: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:164 - TPM2_PCR_CreatePrimary Command
    /// Create Primary Key.
    /// This includes Storate Root Keys and Attestation Identity Keys.
    fn tpm_create_primary(
        &self,
        locality: u32,
        pcr_idx: Option<usize>,
        unique_base: &[u8],
        restricted: bool,
        decrypt: bool,
        sign: bool,
        parent_handle: &mut u32,
        pubkey_size: &mut usize,
        pubkey: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:15 - TPM2_StartAuthSession Command
    /// Start Authenticated Session and returns a session handle
    fn tpm_start_auth_session(
        &self,
        locality: u32,
        session_type: TpmSE,
        nonce: Vec<u8>,
        session_handle: &mut u32,
    ) -> bool;

    /// Table 3:132 - TPM2_PolicyPCR Command
    /// Bind a policy to a particular PCR
    fn tpm_policy_pcr(
        &self,
        locality: u32,
        session_handle: u32,
        digest: Vec<u8>,
        pcr_idx: usize,
    ) -> bool;

    /// Table 3:156 - TPM2_PolicyGetDigest Command
    /// Get Policy digest from current policy
    fn tpm_policy_get_digest(
        &self,
        locality: u32,
        session_handle: u32,
        policy_digest: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:19 - TPM2_Create Command
    /// Create child key
    fn tpm_create(
        &self,
        locality: u32,
        pcr_idx: Option<usize>,
        parent_handle: u32,
        policy: Vec<u8>,
        sensitive_data: Vec<u8>,
        restricted: bool,
        decrypt: bool,
        sign: bool,
        out_private: &mut Vec<u8>,
        out_public: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:21 - TPM2_Load Command
    /// Load objects into the TPM.
    /// The TPM2B_PUBLIC and TPM2B_PRIVATE objects created by the TPM2_Create command
    /// are to be loaded.
    fn tpm_load(
        &self,
        locality: u32,
        parent_handle: u32,
        in_private: Vec<u8>,
        in_public: Vec<u8>,
        item_handle: &mut u32,
    ) -> bool;

    /// Table 3:31 - TPM2_Unseal Command
    /// Unseal data sealed via TPM_CC_CREATE
    fn tpm_unseal(
        &self,
        locality: u32,
        session_handle: u32,
        item_handle: u32,
        out_data: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:90 - TPM2_Quote
    /// Generate Quote.
    /// Since the communication channel between the process and the TPM is untrusted,
    /// TPM2_Quote should be the command to retreive PCR values, not TPM2_PCR_Read
    fn tpm_quote(
        &self,
        locality: u32,
        handle: u32,
        hash: u16,
        nonce: Vec<u8>,
        pcr_idxs: Vec<usize>,
        out_pcr_digest: &mut Vec<u8>,
        out_sig: &mut Vec<u8>,
    ) -> bool;

    /// Table 3:198 - TPM2_FlushContext Command
    /// Remove loaded objects, sequence objects, and/or sessions from TPM memory
    fn tpm_flush_context(&self, locality: u32, flush_handle: u32) -> bool;
}
