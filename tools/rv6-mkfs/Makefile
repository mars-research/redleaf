# Adapted from original xv6 mkfs Makefile

root := ../../
DEBUG ?= false

RV6_BIN = $(root)/domains/usr/xv6/usr/bin

all: build/libfs.a

build/mkfs:
	mkdir -p build && cargo build --$(PROFILE)

# Prevent deletion of intermediate files, e.g. cat.o, after first build, so
# that disk image changes after first build are persistent until clean.  More
# details:
# http://www.gnu.org/software/make/manual/html_node/Chained-Rules.html
.PRECIOUS: %.o

HTDOCS_DIR = $(root)/domains/usr/xv6/usr/bin/src/bin/htdocs/
HTDOCS=$(shell ls $(HTDOCS_DIR) -1)

FILES=README.md large

PROFILE ?= release

BINS=$(shell find $(root)/domains/usr/xv6/usr/bin/src/bin -maxdepth 1 -mindepth 1 ! -name htdocs ! -name target -exec basename {} .rs \;)

build/fs.img: build/mkfs $(FILES) $(BINS) $(HTDOCS) Makefile
	cd build && cargo run --$(PROFILE) -- fs.img $(FILES) $(BINS) $(HTDOCS)

build/fs.o: build/fs.img
	objcopy -I binary -O elf64-x86-64 -B i386 build/fs.img build/fs.o

.PHONY: build/libfs.a
build/libfs.a: build/fs.o
	ar cr --target elf64-x86-64 build/libfs.a build/fs.o

.PHONY: debug
debug:
	PROFILE=dev 
	make debug/fs.img

.PHONY: debug/fs.img
debug/fs.img: build/mkfs $(TEST_FILES) Makefile
	TEST_FILES=$(shell echo $(BINS) | grep -oE -- 'ln|ls')
	TEST_FILES+=$(shell echo $(HTDOCS) | grep -oe 'style.css')
	mkdir -p test && cd test && cargo run --$(PROFILE) -- fs.img $(TEST_FILES)

.PHONY: README.md
README.md:
	cp README.md build/

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

.PHONY: $(TEST_FILES)
$(TEST_FILES): 
	cp $@ test/ -f

.PHONY: clean
clean: 
	rm -rf build/
	cargo clean

.PHONY: clean_test
clean_test:
	rm -rf test/
	cargo clean