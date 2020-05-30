root := ../../../../
include $(root)/common_flags.mk

bin_name := sh
arch ?= x86_64
target := $(arch)-redleaf-domain
bin := target/$(target)/$(TARGET_SUB_DIR)/$(bin_name)

.PHONY: all
all: bin checkstack

.PHONY: release
release: $(releaseinit)

.PHONY: clean
clean:
	rm -rf build
	cargo clean

.PHONY: bin
bin:
	@REDLEAF_ROOT=$(shell realpath $(root)) cargo ${CARGO_COMMAND} ${CARGO_FLAGS} $(FEATURES)

include $(root)/checkstack.mk
