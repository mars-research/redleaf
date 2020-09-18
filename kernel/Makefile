.PHONY: all
all: build/libredleaf.a build/entry.o build/entryother.bin build/multiboot_header.o build/boot.o

.PHONY: clean
clean:
	cargo clean
	rm -rf build

# compile assembly files for the exception entry code
build/entry.o: src/arch/entry_64.S 
	@mkdir -p build
	gcc -fno-builtin -fno-strict-aliasing -Wall -MD -ggdb -fno-pic -nostdinc -I. -o build/entry.o -c src/arch/entry_64.S

.PHONY: build/libredleaf.a
build/libredleaf.a:
	@BUILD_VERSION="$(shell date) ($(shell whoami)@$(shell hostname))" RUST_TARGET_PATH="$(shell pwd)" RUSTFLAGS="-Z emit-stack-sizes" cargo build --out-dir=build -Z build-std=core,alloc -Z features=host_dep -Z unstable-options $(CARGO_FLAGS) $(KERNEL_FEATURES)

build/entryother.bin: src/entryother.asm
	@mkdir -p build
	nasm -felf64 src/entryother.asm -o build/entryother.o
	ld -N -e start_others16 -Ttext 0x7000 -o build/entryother.out build/entryother.o
	objcopy -S -O binary -j .text build/entryother.out build/entryother.bin

build/multiboot_header.o: src/multiboot_header.asm
	@mkdir -p build
	nasm -felf64 src/multiboot_header.asm -o build/multiboot_header.o

build/boot.o: src/boot.asm
	@mkdir -p build
	nasm -felf64 src/boot.asm -o build/boot.o
