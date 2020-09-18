# RedLeaf

![](https://github.com/mars-research/redleaf/workflows/build/badge.svg)

RedLeaf is a research operating system developed from scratch in Rust to explore the impact of language safety on operating system organization.

## Building

You need a very recent nightly Rust toolchain. If you wish to generate a bootable `.iso` image (required for the `qemu` targets), you also need to have `grub-mkrescue` in your PATH.

```
make kernel     # Build kernel proper
make domains    # Build domains
make fatmb2     # Build redleaf.mb2 (Multiboot v2), with kernel and all domains ("fat")
make iso        # Build redleaf.iso
make qemu       # Build and launch QEMU
make qemu-kvm   # Build and launch QEMU with KVM
```

For the `qemu` targets, specify `GDB=1` to start a GDB server and pause execution on boot.

## Foliage

Foliage is a multi-purpose tool that helps you inspect domain dependencies, validate their safeness, and discover potential pitfalls.
You can invoke the tool at the project root with `./fo`.

For example, to view information about the `ixgbe` domain, run:
```
./fo crate ixgbe
```
