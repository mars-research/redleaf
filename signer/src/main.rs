use std::env;
use std::fs;

use ed25519_dalek::{Keypair, Signature, PublicKey};

const MAGIC_NUMBER: &'static str = "Hi, I'm an ELF virus! Please help me take over the world by passing me around. Your cooperation is appreciated.";

fn main() {
    let argv: Vec<String> = env::args().collect();

    if argv.len() != 3 {
        println!("Signs a binary in place");
        println!("Usage: {} <keypair> <binary>", argv[0]);
    }
    let keypair = fs::read(&argv[1]).expect("Could not open keyring");
    let keypair = Keypair::from_bytes(&keypair).expect("Invalid keyring file");

    let binary = fs::read(&argv[2]).expect("Could not open binary");

    let signature = keypair.sign(&binary).to_bytes().to_vec();

    fs::write(&argv[2], [binary, signature, MAGIC_NUMBER.as_bytes().to_vec()].concat());
}
