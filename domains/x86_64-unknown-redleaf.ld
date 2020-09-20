OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(trusted_entry)

SECTIONS {
  . = 0x0; /* Link at 0x0 */

  /* . = 0x227000;  */

  .rodata :
  {
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text :
  {
    __text_start = .;
    *(.text .text.*);
    . = ALIGN(4K);
    __text_end = .;
  }

  .rodata :
  {
     __rodata_start = .;
     *(.rodata*)
     . = ALIGN(4096);
     __rodata_end = .;
  }

  .data :
  {
    __data_start = .;
    *(.data .data.*)
    . = ALIGN(4K);
    __data_end = .;
  }

  .bss :
  {
    __bss_start = .;
    *(.bss .bss.*)
    . = ALIGN(4K);
    __bss_end = .;
  }

  .tdata :
  {
        __tdata_start = .;
        *(.tdata*)
        . = ALIGN(4096);
        __tdata_end = .;
        __tbss_start = .;
        *(.tbss*)
        . += 8;
        . = ALIGN(4096);
        __tbss_end = .;
  }

  .got :
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt :
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : ALIGN(4K) {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : ALIGN(4K) {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }

  /* `INFO` makes the section not allocatable so it won't be loaded into memory */
  .stack_sizes (INFO) :
  {
    KEEP(*(.stack_sizes));
  }

  __end = .;

}
