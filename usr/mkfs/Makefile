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
BUILD_DIR = build/

all: $(BUILD_DIR)/fs.img
	
$(BUILD_DIR)/mkfs: mkfs.c fs.h param.h stat.h types.h
	mkdir $(BUILD_DIR)
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
	benchfs\
	benchnet\
	benchnvme\

$(BUILD_DIR)/fs.img: $(BUILD_DIR)/mkfs $(FILES) $(BINS) Makefile
	mkdir -p build
	cd $(BUILD_DIR) && ./mkfs fs.img $(FILES) $(BINS)

$(BUILD_DIR)/fs.o: $(BUILD_DIR)/fs.img
	objcopy -I binary -O elf64-x86-64 -B i386 $(BUILD_DIR)/fs.img $(BUILD_DIR)/fs.o

$(BUILD_DIR)/libfs.a: $(BUILD_DIR)/fs.o
	ar cr --target elf64-x86-64 $(BUILD_DIR)/libfs.a $(BUILD_DIR)/fs.o

.PHONY: README
README:
	cp README $(BUILD_DIR)/

.PHONY: large
large:
	echo "hello" > $(BUILD_DIR)/$@
 	# wget https://ocw.mit.edu/ans7870/6/6.006/s08/lecturenotes/files/t8.shakespeare.txt -O $@
	# fallocate -l 128M $@

.PHONY: bin
bin:
	make -C ../xv6/usr/bin

.PHONY: $(BINS)
$(BINS): bin
	cp ../xv6/usr/bin/target/$(target)/$(TARGET_SUB_DIR)/$@ $(BUILD_DIR) -f

.PHONY: clean
clean: 
	rm -rf build
