root := ../../../
include $(root)/common_flags.mk

bin_name := ahci_driver
arch ?= x86_64
target := $(arch)-redleaf-domain
bin := build/$(bin_name)
rbin := target/$(target)/$(TARGET_SUB_DIR)/$(bin_name)

target_dir ?= $(root)/src/targets/$(target)
target_spec ?= $(target_dir)/$(target).json
target_link ?= $(target_dir)/linker.ld

.PHONY: all
all: $(bin) checkstack

.PHONY: release
release: $(releaseinit)

.PHONY: clean
clean:
	rm -rf build
	cargo clean

.PHONY: $(bin)
$(bin):
	@RUST_TARGET_PATH=$(shell pwd) RUSTFLAGS="-Z emit-stack-sizes -C link-args=-T$(target_link)" cargo ${CARGO_COMMAND} ${CARGO_FLAGS} --target $(target_spec) $(FEATURES)
	mkdir -p build
	cp $(rbin) $(bin)

include $(root)/checkstack.mk
