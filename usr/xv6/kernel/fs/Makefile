
bin_name := xv6fs
arch ?= x86_64
bin := build/$(bin_name)

linker_script := linker.ld

target ?= $(arch)-$(bin_name)
lib := target/$(target)/debug/lib$(bin_name).a

.PHONY: all
all: $(bin)

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
	@RUST_TARGET_PATH=$(shell pwd) cargo xbuild --target x86_64-$(bin_name).json
