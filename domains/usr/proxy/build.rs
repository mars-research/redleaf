use std::env::var;
use std::process::Command;

fn main() {
    let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap();

    // let fingerprint = Command::new("find")
    //     .arg(format!("{}/../../../lib/core/interfaces/usr"))
    //     .output();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}/../../../lib/core/interfaces/usr/*", manifest_dir);
    println!("cargo:rerun-if-changed={}/../../../tools/rv6-mkfs/", manifest_dir);
    println!("cargo:rustc-link-search=native={}/../../../tools/rv6-mkfs/build", manifest_dir);
    println!("cargo:rustc-link-lib=static=fs");
}
