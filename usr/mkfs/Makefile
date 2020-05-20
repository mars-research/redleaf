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

all: build/fs.img
	
mkfs: mkfs.c fs.h param.h stat.h types.h
	gcc -Werror -Wall -g -o mkfs mkfs.c

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

build/fs.img: mkfs $(FILES) $(BINS) Makefile
	mkdir -p build
	./mkfs build/fs.img $(FILES) $(BINS)

.PHONY: large
large:
	echo "hello" > $@
 	# wget https://ocw.mit.edu/ans7870/6/6.006/s08/lecturenotes/files/t8.shakespeare.txt -O $@
	# fallocate -l 128M $@

.PHONY: $(BINS)
$(BINS):
	make -C ../xv6/usr/$@
	cp ../xv6/usr/$@/build/$@ . -f

.PHONY: clean
clean: 
	rm -f *.o *.d
	rm -rf build
	rm -f ./mkfs
	rm -f ./init
	rm -f ./sh
	rm -f ./ls
	rm -f ./wc
	rm -f ./benchfs
	rm -f ./large
	rm -f ./benchnet
