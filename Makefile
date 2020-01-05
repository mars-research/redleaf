arch ?= x86_64
bin := build/kernel.bin
iso := build/redleaf.iso
root := ./

linker_script := linker.ld
grub_cfg := boot/grub.cfg
#assembly_source_files := $(wildcard src/*.asm)
#assembly_object_files := $(patsubst src/%.asm, build/%.o, $(assembly_source_files))

#FEATURES = --features "smp"
FEATURES =
target ?= $(arch)-redleaf
rust_os := target/$(target)/debug/libredleaf.a
xv6fs_img = usr/mkfs/build/fs.img
root := ./
domain_list := sys/init/build/init \
	usr/xv6/kernel/core/build/xv6kernel \
	usr/xv6/kernel/fs/build/xv6fs \
	sys/dev/pci/build/pci \
	sys/dev/ahci/build/ahci \
	sys/dev/ixgbe_driver/build/ixgbe_driver \
	usr/xv6/usr/shell/build/shell

qemu_common := -m 512M -vga std -s
qemu_common := $(qemu_common) -cdrom $(iso)
qemu_common := $(qemu_common) -no-reboot -no-shutdown -d int,cpu_reset
qemu_common := $(qemu_common) -drive id=satadisk,file=$(xv6fs_img),if=none
qemu_common := $(qemu_common) -device ahci,id=ahci
qemu_common := $(qemu_common) -device ide-drive,drive=satadisk,bus=ahci.0
qemu_common := $(qemu_common) -smp 4

# https://superuser.com/a/1412150
qemu_nox := -nographic -chardev stdio,id=char0,mux=on,logfile=serial.log,signal=off -serial chardev:char0 -mon chardev=char0

qemu_x := -serial file:serial.log

.PHONY: all
all: $(bin)

.PHONY: release
release: $(releaseKernel)

.PHONY: clean
clean:
	-make -C sys clean
	-make -C usr/xv6 clean
	-rm -rf build
	-cargo clean
	-make -C usr/mkfs clean

.PHONY: run
run: qemu

.PHONY: run-nox
run-nox: qemu-nox

.PHONY: qemu
qemu: $(iso) $(xv6fs_img)
	qemu-system-x86_64 $(qemu_common) $(qemu_x)

.PHONY: qemu-gdb
qemu-gdb: $(iso) $(xv6fs_img)
	qemu-system-x86_64 $(qemu_common) $(qemu_x) -S

.PHONY: qemu-gdb-nox
qemu-gdb-nox: $(iso) $(xv6fs_img)
	qemu-system-x86_64 $(qemu_common) $(qemu_nox) -S

.PHONY: qemu-nox
qemu-nox: $(iso) $(xv6fs_img)
	qemu-system-x86_64 $(qemu_common) $(qemu_nox)

.PHONY: qemu-nox-cloudlab
qemu-nox-cloudlab: $(iso)
	$(eval pciflag := $(shell sudo ./rebind-82599es.sh))
	sudo qemu-system-x86_64 $(qemu_common) $(qemu_nox) $(pciflag)

.PHONY: qemu-efi-nox
qemu-efi-nox: $(iso) $(xv6fs_img) ovmf-code
	qemu-system-x86_64 $(qemu_common) $(qemu_nox) -bios OVMF_CODE.fd

$(xv6fs_img):
	make -C usr/mkfs 

ovmf-code:
	echo "Getting OVMF_CODE.fd is not implemented..."

.PHONY: iso
iso: $(iso)
	@echo "Done"

$(iso): $(bin) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(bin) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(bin): kernel $(rust_os) bootblock entryother entry $(linker_script) init checkstack
	ld -n --gc-sections -T $(linker_script) -o $(bin) build/entry.o build/boot.o build/multiboot_header.o $(rust_os) -b binary build/entryother.bin $(domain_list) 

include $(root)/checkstack.mk

.PHONY: init
init:
	make -C usr/xv6
	make -C sys

.PHONY: kernel
kernel:
	@RUST_TARGET_PATH=$(shell pwd) RUSTFLAGS="-Z emit-stack-sizes" cargo xbuild --target x86_64-redleaf.json $(FEATURES)


# compile assembly files for the exception entry code
.PHONY: entry
entry: src/arch/entry_64.S 
	@mkdir -p build
	gcc -fno-builtin -fno-strict-aliasing -Wall -MD -ggdb -fno-pic -nostdinc -I. -o build/entry.o -c src/arch/entry_64.S


# compile assembly files
.PHONY: bootblock
bootblock: src/boot.asm src/multiboot_header.asm
	@mkdir -p build
	nasm -felf64 src/boot.asm -o build/boot.o
	nasm -felf64 src/multiboot_header.asm -o build/multiboot_header.o

# compile assembly files
.PHONY: entryother
entryother: src/entryother.asm
	@mkdir -p build
	nasm -felf64 src/entryother.asm -o build/entryother.o
	ld -N -e start_others16 -Ttext 0x7000 -o build/entryother.out build/entryother.o
	objcopy -S -O binary -j .text build/entryother.out build/entryother.bin

