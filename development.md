# Some notes to help developing redleaf

## Debugging interrupts
1. `grep` for `v=` when you want to see what interrupt that qemu's sending to the kernel. add `-d int` to qemu flag to ask it to print out interrupt info.
2. The handler for interrupts is `do_IRQ`

## Debugging domains
When `gdb`ing, the symbols for the domains are not automatically loaded.
To load the symbols for your domain, load the symbol file at the address of the start of the text section.
For example, given the following output
```
cpu(0):Starting xv6 kernel
cpu(0):domain/xv6kernel: Binary start: 667cdc, end: 755ba4 
cpu(0):domain/xv6kernel: Binary is unsigned
cpu(0):NYI free
cpu(0):NYI free
cpu(0):num_pages: 15
cpu(0):domain/xv6kernel: Entry point at 103657b70
cpu(0):domain/xv6kernel: .text starts at 103650000
```
, you will need to do `add-symbol-file domains/build/xv6kernel 0x103650000`.
