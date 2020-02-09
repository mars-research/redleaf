#!/bin/sh

echo "Setting up Rust build environemnt for RedLeaf"
curl https://sh.rustup.rs -sSf | sh
USR_HOME=`printenv HOME`
RUST_HOME=$USR_HOME/.cargo/bin
echo $RUST_HOME
$RUST_HOME/rustup override add nightly
$RUST_HOME/rustup component add llvm-tools-preview
$RUST_HOME/cargo install cargo-xbuild
$RUST_HOME/cargo +nightly install stack-sizes
$RUST_HOME/rustup component add rust-src

# Install Qemu
sudo apt-get install qemu

# Install nasm

sudo apt-get install nasm

# Install Grub
sudo apt-get install grub-pc-bin

# Install Xorriso
sudo apt-get install xorriso

echo "To get started you need Cargo's bin directory ($HOME/.cargo/bin) in your PATH environment variable. Next time you log in this will be done automatically."
echo "To configure your current shell run source $HOME/.cargo/env"
