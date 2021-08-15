.DEFAULT_GOAL := mb2

################
# Configurations
################

DEBUG            ?= false
LARGE_MEM        ?= true
IXGBE		 	 ?= true
VIRTIO_NET 		 ?= false
VIRTIO_BLOCK 	 ?= false

ifndef NO_DEFAULT_FLAGS
CARGO_FLAGS      ?=

DOMAIN_FEATURES  ?=
KERNEL_FEATURES  ?=

ifeq ($(DEBUG),false)
CARGO_FLAGS      += --release
endif

#KERNEL_FEATURES += --features "trace_alloc"
#KERNEL_FEATURES += --features "smp"
KERNEL_FEATURES  += --features "trace_vspace"
KERNEL_FEATURES  += --features "page_fault_on_ist"
#KERNEL_FEATURES += --features "trace_sched"

endif # NO_DEFAULT_FLAGS

ifdef IXGBE
$(warning IXGBE is always enabled now.)
endif
DOMAIN_FEATURES  += --features "c220g2_ixgbe"
KERNEL_FEATURES  += --features "c220g2_ixgbe"

ifeq ($(LARGE_MEM),true)
KERNEL_FEATURES  += --features "large_mem"
endif

ifeq ($(BAREMETAL),true)
KERNEL_FEATURES  += --features "baremetal"
endif

################
export CARGO_FLAGS
export DOMAIN_FEATURES
export KERNEL_FEATURES
################

arch            ?= x86_64
mb2             := build/redleaf.mb2
iso             := build/redleaf.iso
root            := ./

linker_script   := linker.ld
grub_cfg        := boot/grub.cfg

xv6fs_img = tools/rv6-mkfs/build/fs.img
root := ./
domain_list := $(addprefix domains/build/, \
	redleaf_init \
	dom_proxy \
	dom_c \
	dom_d \
	shadow \
	xv6kernel \
	xv6fs \
	xv6net \
	xv6net_shadow \
	pci \
	ixgbe \
	virtio_net \
	virtio_block \
	nvme \
	tpm \
	bdev_shadow \
	net_shadow \
	nvme_shadow \
	membdev \
	benchnet_inside \
	benchnvme \
	benchhash)



################
# QEMU
################

qemu_common     := ${QEMU_MEM} -vga std
qemu_common     += -cdrom $(iso)
qemu_common 	+= -boot d
# qemu_common    	+= -no-reboot -no-shutdown -d int,cpu_reset
qemu_common     += -drive id=satadisk,file=$(xv6fs_img),format=raw,if=none
qemu_common     += -device ahci,id=ahci
qemu_common     += -device ide-hd,drive=satadisk,bus=ahci.0
# qemu_common    	+= -smp 4
# qemu_common     	+= -monitor telnet:127.0.0.1:55555,server,nowait
qemu_common     += -cpu 'Haswell,pdpe1gb' -machine q35
# qemu_common    	+= -device vfio-pci,romfile=,host=06:00.1
# qemu_common    	+= -vnc 127.0.0.1:0
# qemu_common		+= -mem-path /dev/hugepages
# qemu_common		+= --trace virtio_*
# qemu_common		+= --trace virtqueue_*
# qemu_common		+= --trace file_*
# qemu_common		+= --trace vhost_*
# qemu_common		+= --trace vfio_*




ifeq ($(LARGE_MEM),true)
qemu_common     += -m 8G
else
qemu_common     += -m 2048M
endif

ifeq ($(TPM),true)
qemu_common     += -chardev socket,id=chrtpm,path=/tmp/mytpm1/swtpm-sock
qemu_common     += -tpmdev emulator,id=tpm0,chardev=chrtpm
qemu_common     += -device tpm-tis,tpmdev=tpm0
endif

ifeq ($(GDB),true)
qemu_common     += -S -s
endif

ifeq ($(VIRTIO_BLOCK),true)
# use xv6 image, requires that AHCI is not using the file
# qemu_common 	+= -drive if=none,id=virtio_block,file=$(xv6fs_img),format=raw,cache=none,aio=native

# use disk.img file
qemu_common 	+= -drive if=none,id=virtio_block,file=disk.img,format=raw
qemu_common 	+= -device virtio-blk-pci,drive=virtio_block,ioeventfd=off

DOMAIN_FEATURES += --features "virtio_block"
# DOMAIN_FEATURES += --features "benchnvme"
endif

ifeq ($(VIRTIO_NET),true)
qemu_common 	+= -device virtio-net-pci,netdev=net0
qemu_common		+= -netdev tap,id=net0,ifname=virtio,script=no,downscript=no
DOMAIN_FEATURES += --features "virtio_net"
endif

QEMU            ?= $(shell which qemu-system-x86_64)
TASKSET         := $(shell which taskset)
KVM             := sudo ${TASKSET} -c 3-4 ${QEMU}
qemu_kvm_args	:= --enable-kvm

# https://superuser.com/a/1412150
# We set the first serial to /dev/null because we want to always use COM2
qemu_nox        := -nographic
qemu_nox        += -chardev stdio,id=char0,mux=on,logfile=serial.log,signal=off -serial file:/dev/null -serial chardev:char0 -mon chardev=char0

qemu_x          := -serial file:/dev/null -serial file:serial.log

.PHONY: run
run: qemu

.PHONY: run-nox
run-nox: qemu-nox

.PHONY: qemu
qemu: $(iso) $(xv6fs_img)
	$(QEMU) $(qemu_common) $(qemu_x)

.PHONY: qemu-nox
qemu-nox: $(iso) $(xv6fs_img)
	$(QEMU) $(qemu_common) $(qemu_nox)

.PHONY: qemu-kvm
qemu-kvm: $(iso) $(xv6fs_img)
	${KVM} $(qemu_common) $(qemu_kvm_args) $(qemu_nox)

.PHONY: qemu-gdb
qemu-gdb:
	NO_DEFAULT_FLAGS=1 make qemu GDB=true

.PHONY: qemu-kvm-gdb
qemu-kvm-gdb:
	NO_DEFAULT_FLAGS=1 make qemu-kvm GDB=true

.PHONY: qemu-gdb-nox
qemu-gdb-nox:
	NO_DEFAULT_FLAGS=1 make qemu-nox GDB=true

.PHONY: qemu-nox-gdb
qemu-nox-gdb:
	NO_DEFAULT_FLAGS=1 make qemu-nox GDB=true

################
# Build
################

.PHONY: kernel
kernel: idl_generation
	make -C kernel

.PHONY: idl_generation
idl_generation: tools/redIDL
	@ if [ ! -f "tools/redIDL/codegen/ngc/Cargo.toml" ]; \
	then echo "redIDL not found. Maybe you want to do 'git submodule init && git submodule update' then try again?"; \
			exit -1; \
	fi
	make -C interface

.PHONY: domains
domains: idl_generation $(xv6fs_img) memops
	make -C domains

mb2: $(mb2) checkstack

.PHONY: check
check:
	cd domains && cargo rcheck && cd ../kernel && cargo rcheck

.PHONY: clean
clean:
	-cargo clean --manifest-path=domains/Cargo.toml
	-cargo clean --manifest-path=kernel/Cargo.toml
	-rm -rf build
	-make -C lib/external/memops clean
	-make -C interface clean

.PHONY: clean-keys
clean-keys:
ifeq ($(I_READ_THE_MAKEFILE), doit)
	shred -u redleaf.key redleaf.pub
else
	$(error mixed implicit and static pattern rules)
endif

.PHONY: install
install:
	NO_DEFAULT_FLAGS=1 make BAREMETAL=true
	sudo cp -v $(mb2) /boot

.PHONY: release
release: $(releaseKernel)

.PHONY: $(xv6fs_img)
$(xv6fs_img):
	make -C tools/rv6-mkfs

.PHONY: iso
iso: $(iso)
	@echo "Done"

$(iso): $(mb2) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(mb2) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(mb2): kernel domains memops $(linker_script)
	mkdir -p $(shell dirname $(mb2))
	ld -n --gc-sections -T $(linker_script) -o $(mb2) \
		kernel/build/entry.o \
		kernel/build/boot.o \
		kernel/build/multiboot_header.o \
		kernel/build/thread.o \
		kernel/build/libredleaf_kernel.a \
		lib/external/memops/libmemops.a \
		-b binary \
		kernel/build/entryother.bin \
		$(domain_list)

.PHONY: memops
memops:
	make -C lib/external/memops

.PHONY: just-run-qemu
just-run-qemu:
	$(QEMU) $(qemu_common) $(qemu_nox)

.PHONY: just-run-qemu-kvm
just-run-qemu-kvm:
	${KVM} $(qemu_common) $(qemu_kvm_args) $(qemu_nox)

.PHONY: create-virtio-tap
create-virtio-tap:
	sudo ip tuntap add mode tap user ${USER} name virtio
	# IP Address for Redleaf Virtio Demo is 10.10.10.10
	sudo ip address add 10.10.10.1/24 dev virtio
	sudo ip link set up virtio

.PHONY: delete-virtio-tap
delete-virtio-tap:
	sudo ip link del virtio


include $(root)/checkstack.mk

#interface-fingerprint: $(shell find sys/interfaces -type f -name "*.rs")
#	$(shell sha512sum sys/interfaces/**.rs | cut -d' ' -f1 | sha512sum | cut -d ' ' -f1 > interface.fingerprint)
