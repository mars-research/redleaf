#!/bin/bash
# Install Rust toolchain and other dependencies.
# WARNING: using this script might mess up your existing rust installation
# Change $INSTALL_HOME if you would like Rust to be installed somewhere else.
# Sample usage:
#   INSTALL_HOME=~/large ./setup.sh 

echo "Setting up Rust build environemnt for RedLeaf"

USR_HOME=`printenv HOME`
INSTALL_HOME=${INSTALL_HOME:-$USR_HOME} # Default install path is $HOME. 
CARGO_HOME=$INSTALL_HOME/.cargo
RUSTUP_HOME=$INSTALL_HOME/.rustup
RUST_HOME=$CARGO_HOME/bin

# Install Rust and Cargo dependencies
echo -e "Cargo home: $CARGO_HOME\nRustup home: $RUSTUP_HOME"
curl https://sh.rustup.rs -sSf | CARGO_HOME=$CARGO_HOME RUSTUP_HOME=$RUSTUP_HOME bash -s -- --default-toolchain nightly-2020-08-22 -y
$RUST_HOME/rustup component add llvm-tools-preview rust-src
$RUST_HOME/cargo install stack-sizes

# # Setup CARGO_HOME and RUSTUP_HOME if using custom installation home
if [ $USR_HOME != $INSTALL_HOME ] &&  [ $(grep -q "CARGO_HOME\|RUSTUP_HOME" ~/.profile) ]
then
    echo -e "\nAdding CARGO_HOME and RUSTUP_HOME to ~/.profile\n"
    echo -e "\nexport CARGO_HOME=$CARGO_HOME\nexport RUSTUP_HOME=$RUSTUP_HOME" | tee -a  ~/.profile
    echo ""
fi

# Install apt dependencies: Qemu, nasm, Grub, Xorriso
sudo apt-get update
sudo apt-get install qemu nasm grub-pc-bin xorriso numactl -y

echo "To get started you need Cargo's bin directory ($CARGO_HOME/bin) in your PATH environment variable. Next time you log in this will be done automatically."
echo "To configure your current shell run 'source $CARGO_HOME/env'"
