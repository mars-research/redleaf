
arch ?= x86_64
kernel := build/kernel.bin
iso := build/redleaf.iso

linker_script := linker.ld
grub_cfg := boot/grub.cfg
#assembly_source_files := $(wildcard src/*.asm)
#assembly_object_files := $(patsubst src/%.asm, build/%.o, $(assembly_source_files))

target ?= $(arch)-redleaf
rust_os := target/$(target)/debug/libredleaf.a

.PHONY: all clean run iso kernel doc disk

all: $(kernel)

release: $(releaseKernel)

clean:
	rm -r build
	cargo clean

# To trace interrupts add: -d int,cpu_reset

run: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log -no-reboot

run-nox: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log -no-reboot -nographic

iso: $(iso)
	@echo "Done"

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) bootblock entryother $(linker_script) 
	ld -n --gc-sections -T $(linker_script) -o $(kernel) build/boot.o build/multiboot_header.o $(rust_os) -b binary build/entry.bin

kernel:
	@RUST_TARGET_PATH=$(32shell pwd) cargo xbuild --target x86_64-redleaf.json

# compile assembly files
bootblock: src/boot.asm src/multiboot_header.asm
	@mkdir -p $(shell dirname build)
	nasm -felf64 src/boot.asm -o build/boot.o
	nasm -felf64 src/multiboot_header.asm -o build/multiboot_header.o

# compile assembly files
entryother: src/entry.asm
	@mkdir -p $(shell dirname build)
	nasm -felf64 src/entry.asm -o build/entry.o
	ld -N -e start_others16 -Ttext 0x7C00 -o build/entry.out build/entry.o
	objcopy -S -O binary -j .text build/entry.out build/entry.bin

