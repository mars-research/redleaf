root := ../../../../
include $(root)/common_flags.mk

bin_name := shell
arch ?= x86_64
bin := build/$(bin_name)
rbin := target/$(target)/$(TARGET_SUB_DIR)/$(bin_name)

target_dir ?= $(root)
target_spec ?= $(target_dir)/$(target).json
target_link ?= $(target_dir)/linker.ld

linker_script := linker.ld

target := $(arch)-redleaf
lib := target/$(target)/$(TARGET_SUB_DIR)/lib$(bin_name).a

.PHONY: all
all: $(bin) checkstack

.PHONY: release
release: $(releaseinit)

.PHONY: clean
clean:
	rm -rf build
	cargo clean

$(bin): lib $(linker_script)
	mkdir -p build
	ld -n -pie --no-dynamic-linker --gc-sections -T $(linker_script) -o $(bin) $(lib)

.PHONY: lib
lib:
	@RUST_TARGET_PATH=$(shell pwd) RUSTFLAGS="-Z emit-stack-sizes" cargo ${CARGO_COMMAND} ${CARGO_FLAGS} --target $(target_spec)

include $(root)/checkstack.mk
