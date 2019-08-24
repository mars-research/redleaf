global start

section .text
bits 32    ; By default, GRUB sets us to 32-bit mode.
start:
    ; Print `OK` to screen
    mov dword [0xb8000], 0x2f4b2f4f
    hlt ; Halt the processor.
