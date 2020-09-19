use std::env::var;
use std::process::Command;

fn main() {
    let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap();

    // A previous version of this build script invokes rv6-mkfs
    // to construct the disk image which in turn depends on
    // rv6 domains under `domains/usr`, blocking the build when
    // building the entire domain workspace.

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}/../../../domains/usr/", manifest_dir);
    println!("cargo:rerun-if-changed={}/../../../tools/rv6-mkfs/", manifest_dir);
    println!("cargo:rustc-link-search=native={}/../../../tools/rv6-mkfs/build", manifest_dir);
    println!("cargo:rustc-link-lib=static=fs");
}
