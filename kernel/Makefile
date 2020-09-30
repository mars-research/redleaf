export BUILD_VERSION="$(shell date) ($(shell whoami)@$(shell hostname))"

.PHONY: all
all: build/libredleaf.a build/entry.o build/entryother.bin build/multiboot_header.o build/boot.o build/thread.o

.PHONY: clean
clean:
	cargo clean
	rm -rf build

.PHONY: check
check:
	cargo rcheck $(CARGO_FLAGS) $(KERNEL_FEATURES)

# compile assembly files for the exception entry code
build/entry.o: src/arch/entry_64.S 
	@mkdir -p build
	gcc -fno-builtin -fno-strict-aliasing -Wall -MD -ggdb -fno-pic -nostdinc -I. -o build/entry.o -c src/arch/entry_64.S

.PHONY: build/libredleaf.a
build/libredleaf.a:
	cargo rbuild $(CARGO_FLAGS) $(KERNEL_FEATURES)

build/entryother.bin: src/entryother.asm
	@mkdir -p build
	nasm -felf64 src/entryother.asm -o build/entryother.o
	ld -N -e start_others16 -Ttext 0x7000 -o build/entryother.out build/entryother.o
	objcopy -S -O binary -j .text build/entryother.out build/entryother.bin

build/%.o: src/%.asm
	@mkdir -p build
	nasm -felf64 $< -o $@
