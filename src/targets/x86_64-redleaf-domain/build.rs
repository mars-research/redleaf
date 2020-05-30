use std::{env, fs};
use std::process::{Command, Output};

fn main() {
    env::var("CARGO").expect("Do not run this manually.");

    let target = env::var("TARGET").expect("TARGET must be set. Are you running this through Cargo?");

    println!("Building domain for {}", target);

    if target != "x86_64-redleaf-domain" {
        println!("Run `cargo build --target=/path/to/x86_64-redleaf-domain.json`");
        panic!("Only x86_64-redleaf-domain is supported :(");
    }

    // Detect RedLeaf root
    let redleaf_root = find_redleaf_root().expect("Failed to detect RedLeaf root. Try setting REDLEAF_ROOT.");

    println!("RedLeaf: {}", redleaf_root);

    // Detect linker script
    let linker_script = format!("{}/src/targets/{}/linker.ld", redleaf_root, target);
    println!("Linker script: {}", linker_script);

    // Copy linker script to PWD
    //
    // This is an ugly hack. There is no sane way to specify the path to a
    // linker script in the target spec JSON, as it's relative to the PWD (the
    // domain's source tree) instead of the JSON itself. We can't set arbitrary
    // rustc flags from the build script, either, per
    // <https://github.com/rust-lang/rfcs/issues/1766>.
    std::fs::copy(linker_script, "linker.ld").unwrap();

    // Generate interface fingerprint
    // FIXME: Calculate the hashes in Rust
    let fingerprint = Command::new("bash")
        .arg("-c")
        .arg(format!("sha512sum {}/sys/interfaces/**.rs | cut -d' ' -f1 | sha512sum | cut -d ' ' -f1", redleaf_root))
        .output();
    let fingerprint = trim_successful_output(fingerprint)
        .expect("Failed to compute interface fingerprint.");

    println!("Interface fingerprint: {}", fingerprint);

    // Embed interface fingerprint
    fs::write("interfaces.fingerprint", fingerprint)
        .expect("Failed to write fingerprint to temporary file");
    Command::new("objcopy")
        .args(&["-I", "binary"])
        .args(&["-O", "elf64-x86-64"])
        .args(&["-B", "i386"])
        .arg("--rename-section=.data=.REDLEAF_INTERFACES,alloc,load,data,contents")
        .arg("interfaces.fingerprint")
        .arg("interfaces.fingerprint.o")
        .output()
        .expect("Failed to generate object file for fingerprint");
}

fn find_redleaf_root() -> Option<String> {
    if let Ok(root) = env::var("REDLEAF_ROOT") {
        return Some(root);
    }

    let git_root = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .output();

    if let Some(root) = trim_successful_output(git_root) {
        return Some(root);
    }

    None
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
