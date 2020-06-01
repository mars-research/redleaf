use std::env::var;
use std::process::Command;

fn main() {
    #[cfg(feature = "musl")]
    {
        let mut command = Command::new("make");
        let output = command
            .args(&["-C", "musl"])
            .arg("ARCH=x86_64")
            .arg("-j")
            .arg("lib/libc.a")
            .output()
            .unwrap();
        assert!(output.status.success(), "Command failed:\n{:?}\n{}", command, String::from_utf8_lossy(&output.stderr));

        let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-search=native={}/musl/lib", manifest_dir);
        println!("cargo:rustc-link-lib=static=c");
    }
}