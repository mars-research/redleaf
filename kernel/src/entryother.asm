global start_others16
global start_others32

section .text

bits 16   ; Start other CPUs in 16 mode

start_others16:
    cli

    ; zero out all segment registers    
    xor  ax, ax
    mov  ds, ax
    mov  es, ax
    mov  ss, ax
    mov  gs, ax
    mov  fs, ax

    ; clear direction flag
    cld

    ;lgdt [gdt32.pointer]
    lgdt [gdt32desc]

    mov  eax, cr0
    ; set protected mode bit
    or   eax, 1
    mov  cr0, eax

    jmp gdt32.code:start_others32

bits 32 
start_others32:
    ; initialize other segments for proper functioning
    mov eax, gdt32.data

    mov ds, eax
    mov es, eax
    mov ss, eax

    ; zero unused segments
    xor eax, eax
    mov fs, eax
    mov gs, eax

    ; we have our ap_stack, new_pgdir (> 4GiB) and start_ap_routine
    ; hidden above start_others16
    mov ebx, start_others16 

    ; jmp to boot.asm's code
    jmp [start_others16 - 32]


gdt32:
    dd 0 ; zero entry
    dd 0
.code: equ $ - gdt32 
    db 0xff
    db 0xff
    db 0
    db 0
    db 0
    db 0x9a
    db 0xcf
    db 0
.data: equ $ - gdt32
    db 0xff
    db 0xff
    db 0
    db 0
    db 0
    db 0x92
    db 0xcf
    db 0
gdt32_end:

;.pointer:
;   dw $ - gdt32 - 1  ; last byte in table
;   dd gdt32          ; start of table

gdt32desc:
   dw gdt32_end - gdt32 - 1; last byte in table
   dd gdt32          ; start of table
