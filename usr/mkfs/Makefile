root := ../../
include $(root)/common_flags.mk

CC = $(TOOLPREFIX)gcc
AS = $(TOOLPREFIX)gas
LD = $(TOOLPREFIX)ld
OBJCOPY = $(TOOLPREFIX)objcopy
OBJDUMP = $(TOOLPREFIX)objdump
CFLAGS = -fno-pic -static -fno-builtin -fno-strict-aliasing -O2 -Wall -MD -ggdb -m32 -Werror -fno-omit-frame-pointer
CFLAGS += $(shell $(CC) -fno-stack-protector -E -x c /dev/null >/dev/null 2>&1 && echo -fno-stack-protector)
ASFLAGS = -m32 -gdwarf-2 -Wa,-divide
# FreeBSD ld wants ``elf_i386_fbsd''
LDFLAGS += -m $(shell $(LD) -V | grep elf_i386 2>/dev/null | head -n 1)
arch ?= x86_64
target := $(arch)-redleaf-domain

all: build/libfs.a
	
build/mkfs: mkfs.c fs.h param.h stat.h types.h
	mkdir -p build
	gcc -Werror -Wall -g -o $@ mkfs.c

# Prevent deletion of intermediate files, e.g. cat.o, after first build, so
# that disk image changes after first build are persistent until clean.  More
# details:
# http://www.gnu.org/software/make/manual/html_node/Chained-Rules.html
.PRECIOUS: %.o

FILES=\
	README\
	large\

BINS=\
	init\
	sh\
	ls\
	wc\
	mkdir\
	getpid\
	uptime\
	dump_inode\
	benchfs\
	benchnet\
	benchnvme\

build/fs.img: build/mkfs $(FILES) $(BINS) Makefile
	mkdir -p build
	cd build/ && ./mkfs fs.img $(FILES) $(BINS)

build/fs.o: build/fs.img
	objcopy -I binary -O elf64-x86-64 -B i386 build/fs.img build/fs.o

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
	make -C ../xv6/usr/bin

.PHONY: $(BINS)
$(BINS): bin
	cp ../xv6/usr/bin/target/$(target)/$(TARGET_SUB_DIR)/$@ build/ -f

.PHONY: clean
clean: 
	rm -rf build
