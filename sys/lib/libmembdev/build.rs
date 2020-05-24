use std::env::var;
use std::process::Command;
use std::io::{self, Write};

fn main() {
    let output = Command::new("make")
        .arg("-C")
        .arg("../../../usr/mkfs")
        .arg("build/libfs.a")
        .output()
        .unwrap();

    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}/../../../usr/", manifest_dir);
    println!("cargo:rustc-link-search=native={}/../../../usr/mkfs/build", manifest_dir);
    println!("cargo:rustc-link-lib=static=fs");
}