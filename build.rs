// RedLeaf kernel build script

use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, PublicKey};
use std::{env, fs};
use std::path::Path;
use std::process::{Command, Output};

fn main() {
    env::var("CARGO").expect("Do not run this manually.");

    // Generate interface fingerprint
    // FIXME: Calculate the hashes in Rust
    let fingerprint = Command::new("bash")
        .arg("-c")
        .arg(format!("sha512sum sys/interfaces/**.rs | cut -d' ' -f1 | sha512sum | cut -d ' ' -f1"))
        .output();
    let fingerprint = trim_successful_output(fingerprint)
        .expect("Failed to compute interface fingerprint.");

    println!("cargo:rustc-env=INTERFACE_FINGERPRINT={}", fingerprint);

    // Ensure public key exists and is valid
    let pubkey = {
        if Path::new("redleaf.pub").exists() {
            println!("Using existing keypair");

            let pubkey = fs::read("redleaf.pub").expect("Could not open public key");
            let pubkey = PublicKey::from_bytes(&pubkey).expect("Invalid public key file");

            pubkey
        } else {
            println!("Generating a new keypair");

            let mut csprng = OsRng{};
            let keypair: Keypair = Keypair::generate(&mut csprng);

            std::fs::write("redleaf.key", keypair.to_bytes().to_vec()).expect("Failed to write keyring");
            std::fs::write("redleaf.pub", keypair.public.to_bytes().to_vec()).expect("Failed to write public key");

            keypair.public
        }
    };
    println!("Using public key {:?}", pubkey.to_bytes());
    println!("cargo:rerun-if-changed=redleaf.key");
    println!("cargo:rerun-if-changed=redleaf.pub");
}

fn trim_successful_output(r: std::io::Result<Output>) -> Option<String> {
    if let Ok(output) = r {
        if output.status.success() {
            match String::from_utf8(output.stdout) {
                Ok(s) => {
                    return Some(String::from(s.trim()));
                }
                _ => {}
            }
        }
    }

    None
}
