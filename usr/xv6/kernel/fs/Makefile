root := ../../../../
include $(root)/common_flags.mk

bin_name := xv6fs
arch ?= x86_64
bin := build/$(bin_name)

linker_script := linker.ld

target ?= $(arch)-$(bin_name)
lib := target/$(target)/$(TARGET_SUB_DIR)/lib$(bin_name).a

ifeq ($(MEMFS),true)
FEATURES += --features "memfs"
endif

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
	if [ $(MEMFS) = "true" ]; then\
		ld -n -pie --no-dynamic-linker --gc-sections -T $(linker_script) -o $(bin) $(lib) build/fs.o;\
	else\
        ld -n -pie --no-dynamic-linker --gc-sections -T $(linker_script) -o $(bin) $(lib);\
    fi

.PHONY: lib
lib:
	@RUST_TARGET_PATH=$(shell pwd) RUSTFLAGS="-Z emit-stack-sizes -Z force-overflow-checks" cargo ${CARGO_COMMAND} ${CARGO_FLAGS} --target x86_64-$(bin_name).json ${FEATURES}
	if [ $(MEMFS) = "true" ]; then\
		make -C $(root)/usr/mkfs &&\
        objcopy -I binary -O elf64-x86-64 -B i386 $(root)/usr/mkfs/build/fs.img build/fs.o;\
    fi

include $(root)/checkstack.mk
