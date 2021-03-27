# Adapted from original xv6 mkfs Makefile

root := ../../

RV6_BIN = $(root)/domains/usr/xv6/usr/bin

CC = $(TOOLPREFIX)gcc
AS = $(TOOLPREFIX)gas
LD = $(TOOLPREFIX)ld
OBJCOPY = $(TOOLPREFIX)objcopy
OBJDUMP = $(TOOLPREFIX)objdump
CFLAGS = -fno-pic -static -fno-builtin -fno-strict-aliasing -O2 -Wall -MD -ggdb -Werror -fno-omit-frame-pointer -Wno-error=stringop-truncation
CFLAGS += $(shell $(CC) -fno-stack-protector -E -x c /dev/null >/dev/null 2>&1 && echo -fno-stack-protector)
ASFLAGS = -gdwarf-2 -Wa,-divide
# FreeBSD ld wants ``elf_i386_fbsd''
LDFLAGS += -m $(shell $(LD) -V | grep elf_i386 2>/dev/null | head -n 1)
arch ?= x86_64

all: build/libfs.a


build/mkfs:
	#echo "Rust make" >> oops.txt
	cargo build

# Prevent deletion of intermediate files, e.g. cat.o, after first build, so
# that disk image changes after first build are persistent until clean.  More
# details:
# http://www.gnu.org/software/make/manual/html_node/Chained-Rules.html
.PRECIOUS: %.o

HTDOCS_DIR = $(root)/domains/usr/xv6/usr/bin/src/bin/htdocs/
HTDOCS=$(shell ls $(HTDOCS_DIR) -1)

# HTDOCS=style.css
TEST_DIR=$(shell pwd)/test
TEST_FILES=$(shell ls $(TEST_DIR) -1)

FILES=README large

BINS=$(shell find $(root)/domains/usr/xv6/usr/bin/src/bin -maxdepth 1 -mindepth 1 ! -name htdocs ! -name target -exec basename {} .rs \;)

build/fs.img: build/mkfs $(FILES) $(BINS) $(HTDOCS) Makefile
	mkdir -p build && cd build && ../target/debug/rv6-mkfs fs.img  $(FILES) $(BINS) $(HTDOCS)

build/fs.o: build/fs.img
	objcopy -I binary -O elf64-x86-64 -B i386 build/fs.img build/fs.o

test/fs.img: build/mkfs $(TEST_FILES) Makefile
	mkdir -p build && cd build && ../target/debug/rv6-mkfs fs.img  $(TEST_FILES)

build/libfs.a: build/fs.o
	ar cr --target elf64-x86-64 build/libfs.a build/fs.o

.PHONY: README
README:
	cp README build/

.PHONY: large
large:
	echo "hello" > build/$@
 	# wget https://ocw.mit.edu/ans7870/6/6.006/s08/lecturenotes/files/t8.shakespeare.txt -O $@
	# fallocate -l 128M build/$@

.PHONY: bin
bin:
	cd $(RV6_BIN) && cargo rb --release

.PHONY: $(BINS)
$(BINS): bin
	cp $(RV6_BIN)/build/$@ build/ -f

.PHONY: HTDOCS
$(HTDOCS):
	cp $(HTDOCS_DIR)/$@ build/ -f

.PHONY: clean
clean: 
	rm -rf build
