![](https://github.com/mars-research/redleaf/workflows/build/badge.svg)

### Development Setup
```bash
curl https://sh.rustup.rs -sSf | sh
rustup override add nightly
rustup component add llvm-tools-preview
cargo install cargo-xbuild
cargo +nightly install stack-sizes
rustup component add rust-src
```

### Benchmark Setup
First, install a new linux kernel from Zhaofang by running
```bash
wget https://github.com/mars-research/redleaf/releases/download/bcache_v2/linux-image-5.6.7-meow_5.6.7-meow-6_amd64.deb
sudo dkpg -i linux-image-5.6.7-meow_5.6.7-meow-6_amd64.deb
```
and use https://make-linux-fast-again.com/ to disable KPTI.

<br/>

Then, disable Hyper Threading and fix CPU frequency to a constant by running
```bash
./disable_hyperthreading.sh && ./constant_freq.sh
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

* Install Grub

```
sudo apt-get install grub-pc-bin
```

* Install Xorriso

```
sudo apt-get install xorriso
```


### Run

```
make run
```
Set `DEBUG=false` to build the project in release mode. Set `CLOUDLAB=true` to set up
the correct hardware parameters for running it on d430 machine on cloudlab.

If you see complains from the `grub-mkrescue`, install the `xorriso` package
```
grub-mkrescue: warning: Your xorriso doesn't support `--grub2-boot-info'. Some features are disabled. Please use xorriso 1.2.9 or later..                                                 
```

### Boot on baremetal from a USB drive

Copy the ISO disk image to the USB stick (make sure to use correct device for the 
USB drive, otherwise you can overwrite your hard disk). You can use lsblk on Ubuntu
to list block devices

```
lsblk
```

For me it's /dev/sda or /dev/sdb but my laptop runs off an NVMe device, so for you 
/dev/sda may very well be your root device, not a USB!

```
sudo dd if=build/os.iso of=/dev/<your_usb_drive> bs=1MB
sync
```

### Boot on baremetal from a Linux partition

``` 
sudo cp build/kernel.bin /boot/
```
Add the following entry to the grub menu list. On a Linux machine this can
be done by adding this to the /etc/grub.d/40_custom. You might adjust the
root='hd0,2' to reflect where your Linux root is on disk, e.g., maybe it's on
root='hd0,1'

In the future, we may append different modules to the grub menuentry.
Currently,the backtrace initialization code searches for the 
module parameter "redleaf_kernel" to parse the kernel binary.
```
set timeout=30
menuentry "RedLeaf" {
    set kernel='/boot/kernel.bin'
    echo "Loading ${kernel}..."
    multiboot2 ${kernel}
    module2 ${kernel} redleaf_kernel
    boot
}
```

Update grub

```
  sudo update-grub2
```

Reboot and choose the "RedLeaf" entry. Make sure that you can see the grub menu
list by editing /etc/default/grub making sure that GRUB_HIDDEN_TIMEOUT_QUIET is
set to "false". 

```
  GRUB_HIDDEN_TIMEOUT_QUIET=false
```

### Source code layout

| Path                            | Description                                                 |
|---------------------------------|-------------------------------------------------------------|
| [src](src)                      | Kernel                                                      |
| [sys/dev](sys/dev)              | Device drivers                                              |
| [sys/interfaces](sys/interfaces)| Corss domain interfaces. Shared between kernel and domains. |
| [sys/init](sys/init)            | The first domain get created after kernel is booted.        |
| [usr/mkfs](usr/mkfs)            | Make the file system for the rv6 fs.                        |
| [usr/xv6](usr/xv6)              | The rv6 kernel and it's user programs.                      |

### Known issues

* Qemu does not run on certain machines.[#18](https://github.com/mars-research/redleaf/issues/18)

### Notes

The baremetal Rust setup (features, linking, etc. is best describe in https://os.phil-opp.com/set-up-rust/).

A cleaner baremental setup (multi-boot and no dependencies on external tools): https://kernelstack.net/2019-07-13-rust-os-1/

Two versions of Philipp Opper blog: https://os.phil-opp.com (v2) and https://os.phil-opp.com/first-edition/ (v1)

Naked functions for exceptions: https://os.phil-opp.com/first-edition/extra/naked-exceptions/

### Side notes

* If you are using an outdated version of redleaf it won't boot, try `git cherry-pick 11e80df000bc5f4ea49e67d5147ca94a992a4fbd f2e973019e85171fd298e229472a020d93b880aa 7d55606b309486a2a3d17574edae9dbd7ccad836` to apply patches that fixes some hardware issue.
* The device id of the disk on Zhaofeng's PC: `0x8c82`
* `grep` for `v=` when you want to see what interrupt that qemu's sending to the kernel.

### Supported Cloudlab Machines

#### Machines that the disk driver(AHCI driver) supprots

* d430(HDD)
* c240g1(SSD)
* c240g2(SSD)
