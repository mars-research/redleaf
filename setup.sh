#!/bin/sh
# Install Rust toolchain and other dependencies.
# Change $INSTALL_HOME if you would like Rust to be installed somewhere else.
# Sample usage:
#   ./setup.sh INSTALL_HOME=~/large

echo "Setting up Rust build environemnt for RedLeaf"

USR_HOME=`printenv HOME`
INSTALL_HOME=$USR_HOME # Default install path is $HOME. 
CARGO_HOME=$INSTALL_HOME/.cargo
RUSTUP_HOME= $INSTALL_HOME/.rustup
RUST_HOME=$CARGO_HOME/bin

echo $RUST_HOME
curl https://sh.rustup.rs -sSf | bash -s -- --default-toolchain nightly-2020-08-22 -y
$RUST_HOME/rustup component add llvm-tools-preview rust-src
$RUST_HOME/cargo install stack-sizes

# Install Qemu, nasm, Grub, Xorriso
sudo apt-get update
sudo apt-get install qemu nasm grub-pc-bin xorriso numactl -y

echo "To get started you need Cargo's bin directory ($CARGO_HOME/bin) in your PATH environment variable. Next time you log in this will be done automatically."
echo "To configure your current shell run `source $CARGO_HOME/env`"
