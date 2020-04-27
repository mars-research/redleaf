arch ?= x86_64
bin := build/kernel.bin
iso := build/redleaf.iso
root := ./
include $(root)/common_flags.mk

linker_script := linker.ld
grub_cfg := boot/grub.cfg
#assembly_source_files := $(wildcard src/*.asm)
#assembly_object_files := $(patsubst src/%.asm, build/%.o, $(assembly_source_files))

FEATURES =
#FEATURES += --features "trace_alloc"
FEATURES += --features "smp"
FEATURES += --features "trace_vspace"
FEATURES += --features "page_fault_on_ist"
#FEATURES += --features "trace_sched"

ifeq ($(LARGE_MEM),true)
FEATURES += --features "large_mem"
endif

ifeq ($(IXGBE),true)
FEATURES += --features "c220g2_ixgbe"
PCI_FEATURES += --features "c220g2_ixgbe"
endif

export PCI_FEATURES

target ?= $(arch)-redleaf
rust_os := target/$(target)/$(TARGET_SUB_DIR)/libredleaf.a
xv6fs_img = usr/mkfs/build/fs.img
root := ./
domain_list := sys/init/build/init \
	usr/proxy/build/dom_proxy \
	usr/test/dom_a/build/dom_a \
	usr/test/dom_b/build/dom_b \
	usr/xv6/kernel/core/build/xv6kernel \
	usr/xv6/kernel/fs/build/xv6fs \
	sys/dev/pci/build/pci \
	sys/dev/ahci_driver/build/ahci_driver \
	sys/dev/ixgbe_driver/build/ixgbe_driver \
	usr/xv6/usr/shell/build/shell

qemu_common := -m 2048M -vga std -s
qemu_common += -cdrom $(iso)
qemu_common += -no-reboot -no-shutdown -d int,cpu_reset
qemu_common += -drive id=satadisk,file=$(xv6fs_img),if=none
qemu_common += -device ahci,id=ahci
qemu_common += -device ide-drive,drive=satadisk,bus=ahci.0
qemu_common += -cpu Haswell-IBRS
qemu_common += -smp 4
qemu_common += -monitor telnet:127.0.0.1:55555,server,nowait
qemu_common += -cpu Icelake-Server-v2 -machine q35

QEMU := qemu-system-x86_64
QEMU_KVM := sudo qemu-system-x86_64
qemu_kvm_args := $(qemu_common) --enable-kvm

# https://superuser.com/a/1412150
# We set the first serial to /dev/null because we want to always use COM2
qemu_nox := -nographic -chardev stdio,id=char0,mux=on,logfile=serial.log,signal=off -serial file:/dev/null -serial chardev:char0 -mon chardev=char0

qemu_x := -serial file:/dev/null -serial file:serial.log

.PHONY: all
all: $(bin) checkstack

install: all
	sudo cp -v build/kernel.bin /boot

.PHONY: release
release: $(releaseKernel)

.PHONY: clean
clean:
	-make -C sys clean
	-make -C usr/xv6 clean
	-rm -rf build
	-cargo clean
	-make -C usr/mkfs clean
	-make -C usr/proxy clean
	-make -C usr/test clean

.PHONY: clean-keys
clean-keys:
ifeq ($(I_READ_THE_MAKEFILE), doit)
	shred -u redleaf.key redleaf.pub
else
	$(error mixed implicit and static pattern rules)
endif

.PHONY: run
run: qemu

.PHONY: run-nox
run-nox: qemu-nox

.PHONY: qemu
qemu: $(iso) $(xv6fs_img)
	$(QEMU) $(qemu_common) $(qemu_x)

.PHONY: qemu-kvm
qemu-kvm: $(iso) $(xv6fs_img)
	${QEMU_KVM} $(qemu_kvm_args) $(qemu_nox)

.PHONY: qemu-gdb
qemu-gdb: $(iso) $(xv6fs_img)
	$(QEMU) $(qemu_common) $(qemu_x) -S

.PHONY: qemu-kvm-gdb
qemu-kvm-gdb: $(iso) $(xv6fs_img)
	${QEMU_KVM} $(qemu_kvm_args) $(qemu_nox) -S


.PHONY: qemu-gdb-nox
qemu-gdb-nox: $(iso) $(xv6fs_img)
	$(QEMU) $(qemu_common) $(qemu_nox) -S

.PHONY: qemu-nox
qemu-nox: $(iso) $(xv6fs_img)
	$(QEMU) $(qemu_common) $(qemu_nox)

.PHONY: qemu-nox-cloudlab
qemu-nox-cloudlab: $(iso)
	$(eval pciflag := $(shell sudo ./rebind-82599es.sh))
	sudo $(QEMU) $(qemu_common) $(qemu_nox) $(pciflag)

.PHONY: qemu-efi-nox
qemu-efi-nox: $(iso) $(xv6fs_img) ovmf-code
	$(QEMU) $(qemu_common) $(qemu_nox) -bios OVMF_CODE.fd

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

$(bin): kernel $(rust_os) bootblock entryother entry $(linker_script) init signer
	for elf in $(domain_list); do \
	    signer/signer redleaf.key $$elf; \
	done
	ld -n --gc-sections -T $(linker_script) -o $(bin) build/entry.o build/boot.o build/multiboot_header.o $(rust_os) -b binary build/entryother.bin $(domain_list) 

include $(root)/checkstack.mk

.PHONY: init
init:
	make -C usr/proxy
	make -C usr/test
	make -C usr/xv6
	make -C sys

.PHONY: signer
signer:
	make -C signer

.PHONY: kernel
kernel:
	@BUILD_VERSION="$(shell date) ($(shell whoami)@$(shell hostname))" RUST_TARGET_PATH="$(shell pwd)" RUSTFLAGS="-Z emit-stack-sizes" cargo ${CARGO_COMMAND} ${CARGO_FLAGS} -Z features=host_dep --target x86_64-redleaf.json $(FEATURES)

interface-fingerprint: $(shell find sys/interfaces -type f -name "*.rs")
	$(shell sha512sum sys/interfaces/**.rs | cut -d' ' -f1 | sha512sum | cut -d ' ' -f1 > interface.fingerprint)

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

.PHONY: cloudlab-deps
cloudlab-deps:
	sudo apt update
	sudo apt install -y qemu nasm xorriso
	curl https://sh.rustup.rs -sSf | bash -s -- --default-toolchain nightly -y
	cargo install cargo-xbuild
	cargo install stack-sizes
	rustup component add rust-src

.PHONY: cloudlab-grub
cloudlab-grub:
	cat cloudlab-grub.template | envsubst '$$PWD' | sudo tee /etc/grub.d/40_custom
	sudo update-grub2
