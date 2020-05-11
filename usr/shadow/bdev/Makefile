root := ../../../
include $(root)/common_flags.mk

bin_name := bdev_shadow
arch ?= x86_64
bin := build/$(bin_name)

linker_script := linker.ld

target ?= $(arch)-$(bin_name)
lib := target/$(target)/$(TARGET_SUB_DIR)/lib$(bin_name).a

#FEATURES = --features "test_threads"

.PHONY: all
all: $(bin) checkstack

.PHONY: release
release: $(releaseinit)

.PHONY: clean
clean:
	rm -rf build
	cargo clean

$(bin): lib $(linker_script) fs
	mkdir -p build
	ld -n -pie --no-dynamic-linker --gc-sections -T $(linker_script) -o $(bin) $(lib) build/fs.o

.PHONY: lib
lib:
	@RUST_TARGET_PATH=$(shell pwd) RUSTFLAGS="-Z emit-stack-sizes" cargo ${CARGO_COMMAND} ${CARGO_FLAGS} --target x86_64-$(bin_name).json $(FEATURES)

.PHONY: fs
fs:
	make -C $(root)/usr/mkfs
	mkdir -p build
	objcopy -I binary -O elf64-x86-64 -B i386 $(root)/usr/mkfs/build/fs.img build/fs.o

include $(root)/checkstack.mk
