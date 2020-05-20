use std::env::var;
use std::process::Command;

fn main() {
    #[cfg(feature = "musl")]
    {
        let output = Command::new("make")
            .arg("-C")
            .arg("musl")
            .arg("ARCH=x86_64")
            .arg("-j")
            .arg("musl")
            .output()
            .unwrap()
            .stdout;
    
        let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-search=native={}/musl/lib", manifest_dir);
        println!("cargo:rustc-link-lib=static=c");
    }
}