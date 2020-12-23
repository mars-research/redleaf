# RedLeaf

![](https://github.com/mars-research/redleaf/workflows/build/badge.svg)
![](https://github.com/mars-research/redleaf/workflows/test/badge.svg)

RedLeaf is a research operating system developed from scratch in Rust to explore the impact of language safety on operating system organization.

## Setup

### Nix

The easiest way to set up a reproducible environment with all dependencies is with [Nix](https://nixos.org/download.html).
With Nix installed, simply enter the provided nix-shell environment with:

```
nix-shell --pure
```

### Bash Script

There is also a bash script meant for Debian-based distributions and does not ensure reproducible builds:

``` bash
./setup.sh
```

## Building

You need a very recent nightly Rust toolchain with the `rust-src` component, and also the `nasm` assembler.
If you wish to generate a bootable `.iso` image (required for the `qemu` targets), you also need to have `grub-mkrescue` in your PATH.

```
make check      # Verify that the tree is buildable
make kernel     # Build kernel proper
make domains    # Build domains
make mb2        # Build Multiboot v2 kernel image (redleaf.mb2)
make iso        # Build bootable ISO (redleaf.iso)
make qemu       # Build and launch QEMU
make qemu-nox   # Build and launch QEMU in headless mode
make qemu-kvm   # Build and launch QEMU with KVM
```

For the `qemu` targets, specify `GDB=true` to start a GDB server and pause execution on boot.
By default, the build system will build everything in the `release` mode with optimizations enabled, and you can override this behavior by passing `DEBUG=true`.

## Foliage

Foliage is a multi-purpose tool that helps you inspect domain dependencies, validate their safeness, and discover potential pitfalls.
You can invoke the tool at the project root with `./fo`.

For example, to view information about the `ixgbe` domain, run:
```
./fo crate ixgbe
```
