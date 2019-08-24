curl https://sh.rustup.rs -sSf | sh
rustup override add nightly
rustup component add llvm-tools-preview
cargo install cargo-xbuild
cargo xbuild --target x86_64-redleaf.json 
rustup component add rust-src
cargo xbuild --target x86_64-redleaf.json 

