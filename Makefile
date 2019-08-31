
arch ?= x86_64
kernel := build/kernel.bin
iso := build/redleaf.iso

linker_script := linker.ld
grub_cfg := boot/grub.cfg
assembly_source_files := $(wildcard src/*.asm)
assembly_object_files := $(patsubst src/%.asm, build/%.o, $(assembly_source_files))

target ?= $(arch)-redleaf
rust_os := target/$(target)/debug/libredleaf.a

.PHONY: all clean run iso kernel doc disk

all: $(kernel)

release: $(releaseKernel)

clean:
	rm -r build
	cargo clean

run: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log -d int,cpu_reset -no-reboot

iso: $(iso)
	@echo "Done"

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	ld -n --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_os)

kernel:
	@RUST_TARGET_PATH=$(32shell pwd) cargo xbuild --target x86_64-redleaf.json

# compile assembly files
build/%.o: src/%.asm
	@mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@

