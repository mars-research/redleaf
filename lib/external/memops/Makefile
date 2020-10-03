libmemops.a: musl/lib/libc.a
	strip $< -o $@ -K memcpy -K memmove -K memcmp -K memset
	ranlib $@

musl/lib/libc.a: musl/VERSION
	make -C musl CC=clang ARCH=x86_64 -j lib/libc.a
