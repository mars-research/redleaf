
arch ?= x86_64
bin := build/init

linker_script := linker.ld

target ?= $(arch)-init
lib := target/$(target)/debug/libinit.a

.PHONY: all
all: $(bin)

.PHONY: release
release: $(releaseinit)

.PHONY: clean
clean:
	rm -r build
	cargo clean

$(bin): lib $(linker_script)
	mkdir -p build	
	ld -n --gc-sections -T $(linker_script) -o $(bin) $(lib) 

.PHONY: lib
lib:
	@RUST_TARGET_PATH=$(32shell pwd) cargo xbuild --target x86_64-init.json

