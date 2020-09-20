arch ?= x86_64
bin := build/redleaf.mb2
iso := build/redleaf.iso
root := ./

linker_script := linker.ld
grub_cfg := boot/grub.cfg

# Configurations
CARGO_FLAGS     ?=
DOMAIN_FEATURES ?=
KERNEL_FEATURES ?=
DEBUG           ?= false
LARGE_MEM       ?= true

#KERNEL_FEATURES += --features "trace_alloc"
#KERNEL_FEATURES += --features "smp"
KERNEL_FEATURES += --features "trace_vspace"
KERNEL_FEATURES += --features "page_fault_on_ist"
#KERNEL_FEATURES += --features "trace_sched"

ifeq ($(DEBUG),false)
	CARGO_FLAGS += --release
	TARGET_SUB_DIR = release
endif

ifeq ($(LARGE_MEM),true)
KERNEL_FEATURES += --features "large_mem"
QEMU_MEM = -m 10240M
else
QEMU_MEM = -m 2048M
endif

#ifeq ($(IXGBE),true)
DOMAIN_FEATURES += --features "c220g2_ixgbe"
KERNEL_FEATURES += --features "c220g2_ixgbe"
#endif

export CARGO_FLAGS
export DOMAIN_FEATURES
export KERNEL_FEATURES

xv6fs_img = tools/rv6-mkfs/build/fs.img
root := ./
domain_list := $(addprefix domains/build/, \
	redleaf_init \
	dom_proxy \
	dom_a \
	dom_b \
	dom_c \
	dom_d \
	shadow \
	xv6kernel \
	xv6fs \
	pci \
	ixgbe \
	nvme \
	tpm \
	bdev_shadow \
	net_shadow \
	nvme_shadow \
	membdev \
	benchnet_inside \
	benchnvme \
	benchhash)

qemu_common := ${QEMU_MEM} -vga std -s
qemu_common += -cdrom $(iso)
#qemu_common += -no-reboot -no-shutdown -d int,cpu_reset
qemu_common += -drive id=satadisk,file=$(xv6fs_img),format=raw,if=none
qemu_common += -device ahci,id=ahci
qemu_common += -device ide-hd,drive=satadisk,bus=ahci.0
#qemu_common += -smp 4
qemu_common += -monitor telnet:127.0.0.1:55555,server,nowait
qemu_common += -cpu 'Haswell,pdpe1gb' -machine q35
qemu_common += -net nic,model=virtio
#qemu_common += -device vfio-pci,romfile=,host=06:00.1
#qemu_common += -vnc 127.0.0.1:0

ifeq ($(TPM),true)
qemu_common += -chardev socket,id=chrtpm,path=/tmp/mytpm1/swtpm-sock
qemu_common += -tpmdev emulator,id=tpm0,chardev=chrtpm
qemu_common += -device tpm-tis,tpmdev=tpm0
endif

QEMU := qemu-system-x86_64
QEMU_KVM := sudo numactl -C 4 ${QEMU}
qemu_kvm_args := $(qemu_common) -enable-kvm

# https://superuser.com/a/1412150
# We set the first serial to /dev/null because we want to always use COM2
qemu_nox := -nographic -chardev stdio,id=char0,mux=on,logfile=serial.log,signal=off -serial file:/dev/null -serial chardev:char0 -mon chardev=char0

qemu_x := -serial file:/dev/null -serial file:serial.log

.PHONY: all
all: $(bin) checkstack

.PHONY: kernel
kernel:
	make -C kernel

.PHONY: domains
domains: $(xv6fs_img) memops
	make -C domains

.PHONY: install
install: all
	sudo cp -v build/kernel.bin /boot

.PHONY: release
release: $(releaseKernel)

.PHONY: clean
clean:
	-cargo clean --manifest-path=domains/Cargo.toml
	-cargo clean --manifest-path=kernel/Cargo.toml
	-rm -rf build

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

.PHONY: $(xv6fs_img)
$(xv6fs_img):
	make -C tools/rv6-mkfs

.PHONY: iso
iso: $(iso)
	@echo "Done"

$(iso): $(bin) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(bin) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(bin): kernel domains memops $(linker_script)
	mkdir -p $(shell dirname $(bin))
	ld -n --gc-sections -T $(linker_script) -o $(bin) \
		kernel/build/entry.o \
		kernel/build/boot.o \
		kernel/build/multiboot_header.o \
		kernel/build/libredleaf_kernel.a \
		lib/external/memops/libmemops.a \
		-b binary \
		kernel/build/entryother.bin \
		$(domain_list) 

include $(root)/checkstack.mk

.PHONY: memops
memops:
	make -C lib/external/memops

#interface-fingerprint: $(shell find sys/interfaces -type f -name "*.rs")
#	$(shell sha512sum sys/interfaces/**.rs | cut -d' ' -f1 | sha512sum | cut -d ' ' -f1 > interface.fingerprint)
