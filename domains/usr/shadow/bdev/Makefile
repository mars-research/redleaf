root := ../../../
include $(root)/common_flags.mk

bin_name := bdev_shadow
arch ?= x86_64
target := $(arch)-redleaf-domain
bin := target/$(target)/$(TARGET_SUB_DIR)/$(bin_name)

.PHONY: all
all: $(bin) checkstack

.PHONY: release
release: $(releaseinit)

.PHONY: clean
clean:
	rm -rf build
	cargo clean

.PHONY: $(bin)
$(bin): memfs
	@REDLEAF_ROOT=$(shell realpath $(root)) cargo ${CARGO_COMMAND} ${CARGO_FLAGS} $(FEATURES)

# The build script of libmembdev is not smart enough to rebuild it so we have to rebuild it manually
.PHONY: memfs
memfs:
	make -C $(root)/usr/mkfs

include $(root)/checkstack.mk
