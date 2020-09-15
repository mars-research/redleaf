# polylibc

We use musl to provide implementations of `memcpy`, `memmove`, `memcmp`, and `memset`. We don't link against the entire libc - The archive file is stripped to only contain the symbols we need.
