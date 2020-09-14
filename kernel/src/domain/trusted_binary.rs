use crate::buildinfo;
use signature::{Signature, Verifier};
use ed25519_dalek::PublicKey;

const MAGIC_NUMBER: &'static str = "Hi, I'm an ELF virus! Please help me take over the world by passing me around. Your cooperation is appreciated.";

pub enum SignatureCheckResult {
    Unsigned,
    GoodSignature,
    BadSignature,
}

pub fn verify(binary: &[u8]) -> SignatureCheckResult {
    let pubkey = PublicKey::from_bytes(buildinfo::TRUSTED_SIGNING_KEY).expect("Invalid public key");
    let expected_length = MAGIC_NUMBER.len() + 64;

    if binary.len() < expected_length {
        // Too short
        return SignatureCheckResult::Unsigned;
    }

    let magic_number = &binary[binary.len() - MAGIC_NUMBER.len()..];
    if &magic_number != &MAGIC_NUMBER.as_bytes() {
        // No magic number
        return SignatureCheckResult::Unsigned;
    }

    let sig_start = binary.len() - expected_length;
    let raw_binary = &binary[0..sig_start];
    let raw_signature = &binary[sig_start..sig_start + 64];

    if let Ok(signature) = Signature::from_bytes(raw_signature) {
        if let Ok(_) = pubkey.verify(raw_binary, &signature) {
            return SignatureCheckResult::GoodSignature;
        }
    }

    return SignatureCheckResult::BadSignature;
}
