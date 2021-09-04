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
you will need to do `add-symbol-file domains/build/xv6kernel 0x103650000`.

## Using Vermilion, RedLeaf's GDB Helper
Vermilion is RedLeaf's GDB Helper which automatically loads the symbols for domains into GDB once RedLeaf has loaded them into memory. 
Vermilion will also automatically import all symbols in `domains/build` so that you can use autocomplete when setting your breakpoints. 
(Note that this might cause issues by setting the breakpoint for an incorrect address). 
You don't need to do anything to start Vermilion, it will automatically be loaded thanks to `.gdbinit`.
Please note that Vermilion currently **DOES NOT** support KVM. You must use a command like `make qemu-gdb-nox`.
Aside from that, everything should work *automagically*. Let us know if you encounter any bugs!