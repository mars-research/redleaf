OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(start)

SECTIONS {
  . = 0x100000; /* Tells GRUB to load the kernel starting at the 1MiB mark */

  .rodata :
  {
    /* ensure that the multiboot header is at the beginning */
    KEEP(*(.multiboot_header))
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .entry.text :
  {
    __entry_text_start = .;
    KEEP(*(.entry.text))
    *(.entry.text .entry.text.*)
    . = ALIGN(4K);
    __entry_text_end = .;
  }

  .text :
  {
    __text_start = .;
    *(.text .text.*)
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

  .eh_frame_hdr :
  {
    __eh_frame_hdr_start = .;
    KEEP(*(.eh_frame_hdr));
    . = ALIGN(4K);
    __eh_frame_hdr_end = .;
  }

  .eh_frame :
  {
    __eh_frame_start = .;
    KEEP(*(.eh_frame .eh_frame.*));
    . = ALIGN(4K);
    __eh_frame_end = .;
  }

  .eh_frame :
  {
    __eh_frame_start = .;
    KEEP(*(.eh_frame .eh_frame.*));
    . = ALIGN(4K);
    __eh_frame_end = .;
  }

  .debug_abbrev :
  {
    __debug_abbrev_start = .;
    KEEP(*(.debug_abbrev .debug_abbrev.*));
    . = ALIGN(4K);
    __debug_abbrev_end = .;
  }

  .debug_addr :
  {
    __debug_addr_start = .;
    KEEP(*(.debug_addr .debug_addr.*));
    . = ALIGN(4K);
    __debug_addr_end = .;
  }

  .debug_info :
  {
    __debug_info_start = .;
    KEEP(*(.debug_info .debug_info.*));
    . = ALIGN(4K);
    __debug_info_end = .;
  }

  .debug_loc :
  {
    __debug_loc_start = .;
    KEEP(*(.debug_loc .debug_loc.*));
    . = ALIGN(4K);
    __debug_loc_end = .;
  }

  .debug_line :
  {
    __debug_line_start = .;
    KEEP(*(.debug_line .debug_line.*));
    . = ALIGN(4K);
    __debug_line_end = .;
  }

  .debug_line_str :
  {
    __debug_line_str_start = .;
    KEEP(*(.debug_line_str .debug_line_str.*));
    . = ALIGN(4K);
    __debug_line_str_end = .;
  }

  .debug_ranges :
  {
    __debug_ranges_start = .;
    KEEP(*(.debug_ranges .debug_ranges.*));
    . = ALIGN(4K);
    __debug_ranges_end = .;
  }

  .debug_rnglists :
  {
    __debug_rnglists_start = .;
    KEEP(*(.debug_rnglists .debug_rnglists.*));
    . = ALIGN(4K);
    __debug_rnglists_end = .;
  }

  .debug_str :
  {
    __debug_str_start = .;
    KEEP(*(.debug_str .debug_str.*));
    . = ALIGN(4K);
    __debug_str_end = .;
  }

  .debug_str_offsets :
  {
    __debug_str_offsets_start = .;
    KEEP(*(.debug_str_offsets .debug_str_offsets.*));
    . = ALIGN(4K);
    __debug_str_offsets_end = .;
  }

  /* `INFO` makes the section not allocatable so it won't be loaded into memory */
  .stack_sizes (INFO) :
  {
    KEEP(*(.stack_sizes));
  }
  . = ALIGN(4K);
  __end = .;
}
