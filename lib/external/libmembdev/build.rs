use std::env::var;
use std::process::Command;

fn main() {
    let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}/../../../domains/usr/", manifest_dir);
    println!("cargo:rustc-link-search=native={}/../../../domains/usr/mkfs/build", manifest_dir);
    println!("cargo:rustc-link-lib=static=fs");
}
