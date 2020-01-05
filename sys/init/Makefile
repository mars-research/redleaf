root := ../../

arch ?= x86_64
bin := build/init

linker_script := linker.ld

target ?= $(arch)-init
lib := target/$(target)/debug/libinit.a

FEATURES = 
#FEATURES += --features "test_sleep"
#FEATURES += --features "test_timer_thread" 
#FEATURES += --features "test_threads" 

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
	@RUST_TARGET_PATH=$(shell pwd) RUSTFLAGS="-Z emit-stack-sizes" cargo xbuild --target x86_64-init.json $(FEATURES)

include $(root)/checkstack.mk
