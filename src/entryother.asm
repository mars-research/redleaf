global start_others16
global start_others32
global start_others64
;extern rust_main_others

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
;    jmp 0x08:start_others32


bits 32 
start_others32:
    mov esp, [start_others16 - 4]

    call enable_paging_others 

    ; load the 64-bit GDT
    lgdt [gdt64other.pointer]

    ; jump to long mode / replaces OK code.
    jmp gdt64other.code:start_others64

    hlt ; Halt the processor.

enable_paging_others:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, [start_others16 - 8]
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret


bits 64

start_others64:
    ; load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; print `OKAY` to screen
    ; mov rax, 0x2f592f412f4b2f4f
    ; mov qword [0xb8000], rax

    call [start_others16 - 2*4 - 8]
    hlt

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
.data:
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

gdt64other:
    dq 0 ; zero entry
.code: equ $ - gdt64other
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64other - 1
    dq gdt64other


