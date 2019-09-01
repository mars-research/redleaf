### Setup
```
curl https://sh.rustup.rs -sSf | sh
rustup override add nightly
rustup component add llvm-tools-preview
cargo install cargo-xbuild
rustup component add rust-src
cargo xbuild --target x86_64-redleaf.json
```


### Prerequisites
* Install Qemu

```
sudo apt-get install qemu
```

* Install nasm

```
sudo apt-get install nasm
```

### Run

```
make run
```

If you see complains from the `grub-mkrescue`, install the `xorriso` package
```
grub-mkrescue: warning: Your xorriso doesn't support `--grub2-boot-info'. Some features are disabled. Please use xorriso 1.2.9 or later..                                                 
```

### Boot on baremetal 

``` 
sudo cp build/kernel.bin /boot/
```
Add the following entry to the grub menu list. On a Linux machine this can
be done by adding this to the /etc/grub.d/40_custom. You might adjust the
root='hd0,2' to reflect where your Linux root is on disk, e.g., maybe it's on
root='hd0,1'

```
set timeout=30
menuentry "RedLeaf" {
    insmod ext2
    set root='hd0,1'
    set kernel='/boot/kernel.bin'
    echo "Loading ${kernel}..."
    multiboot2 ${kernel} ${kernel}
    boot
}
```

Update grub

```
  sudo sudo update-grub2
```

Reboot and choose the xv6 entry. Make sure that you can see the grub menu
list by editing /etc/default/grub making sure that GRUB_HIDDEN_TIMEOUT_QUIET is
set to "false". 

```
  GRUB_HIDDEN_TIMEOUT_QUIET=false
```

### Notes

The baremetal Rust setup (features, linking, etc. is best describe in https://os.phil-opp.com/set-up-rust/).

A cleaner baremental setup (multi-boot and no dependencies on external tools): https://kernelstack.net/2019-07-13-rust-os-1/

Two versions of Philipp Opper blog: https://os.phil-opp.com (v2) and https://os.phil-opp.com/first-edition/ (v1)

Naked functions for exceptions: https://os.phil-opp.com/first-edition/extra/naked-exceptions/
