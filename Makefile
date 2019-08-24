
arch ?= x86_64
kernel := build/kernel.bin
iso := build/redleaf.iso

linker_script := linker.ld
grub_cfg := boot/grub.cfg
assembly_source_files := $(wildcard src/*.asm)
assembly_object_files := $(patsubst src/%.asm, build/%.o, $(assembly_source_files))

.PHONY: all clean run iso kernel doc disk

all: $(kernel)

clean:
	rm -r build

run: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -vga std -s -serial file:serial.log

iso: $(iso)
	@echo "Done"

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	cp $(kernel) build/isofiles/boot/kernel.bin
	cp $(grub_cfg) build/isofiles/boot/grub
	grub-mkrescue -o $(iso) build/isofiles #2> /dev/null
	@rm -r build/isofiles

$(kernel): $(assembly_object_files) $(linker_script)
	ld -n -T $(linker_script) -o $(kernel) $(assembly_object_files)

# compile assembly files
build/%.o: src/%.asm
	@mkdir -p $(shell dirname $@)
	nasm -felf64 $< -o $@

