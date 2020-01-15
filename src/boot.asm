global start
global _bootinfo
global start64
extern rust_main

bits 32    ; By default, GRUB sets us to 32-bit mode.
start:
    mov esp, stack_top

    call check_multiboot
    call check_cpuid
    call check_long_mode

    call set_up_page_tables
    call enable_paging 

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    mov word [0xb8000], 0x0248 ; H
    mov word [0xb8002], 0x0265 ; e
    mov word [0xb8004], 0x026c ; l
    mov word [0xb8006], 0x026c ; l
    mov word [0xb8008], 0x026f ; o
    mov word [0xb800a], 0x022c ; ,
    mov word [0xb800c], 0x0220 ;
    mov word [0xb800e], 0x0277 ; w
    mov word [0xb8010], 0x026f ; o
    mov word [0xb8012], 0x0272 ; r
    mov word [0xb8014], 0x026c ; l
    mov word [0xb8016], 0x0264 ; d
    mov word [0xb8018], 0x0221 ; !

    ; jump to long mode / replaces OK code.
    jmp gdt64.code:start64

    hlt ; Halt the processor.

bits 64
start64:
    ; load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    call setup_huge_page_tables

    ; rdmsr IA32_EFER
    mov ecx, 0xc0000080
    rdmsr
    or eax, 1 << 8 ; enable LME bit
    wrmsr

    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov rax, hp4_table
    mov cr3, rax

    ; print `OKAY` to screen
    ; mov rax, 0x2f592f412f4b2f4f
    ; mov qword [0xb8000], rax

    call rust_main
    hlt

setup_huge_page_tables:
    ; map first P4 entry to P3 table
    mov rax, hp3_table
    or rax, 0b11 ; present + writable
    mov [hp4_table], rax

    ;map each P3 entry to a huge 1GiB page
    mov ecx, 0         ; counter variable

.map_hp3_table:
    ; map ecx-th P3 entry to a huge page that starts at address 1GiB*ecx
    mov rax, 1 << 30  ; 1GiB
    mul ecx            ; start address of ecx-th page
    shl rdx, 32
    or rax, rdx
    or rax, 0b10000011 ; present + writable + huge
    mov [hp3_table + ecx * 8], rax ; map ecx-th entry

    inc ecx            ; increase counter
    cmp ecx, 0x20       ; if counter == 32, 32 entries in P3 table is mapped
    jne .map_hp3_table  ; else map the next entry

    ; Apic regions would belong in the first few gigabytes
    ret

bits 32
check_multiboot:
    cmp eax, 0x36d76289 ; If multiboot, this value will be in the eax register on boot.
    mov [_bootinfo], ebx
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error

check_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    ; in the FLAGS register. If we can flip it, CPUID is available.

    ; Copy FLAGS in to EAX via stack
    pushfd
    pop eax

    ; Copy to ECX as well for comparing later on
    mov ecx, eax

    ; Flip the ID bit
    xor eax, 1 << 21

    ; Copy EAX to FLAGS via the stack
    push eax
    popfd

    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the
    ; ID bit back if it was ever flipped).
    push ecx
    popfd

    ; Compare EAX and ECX. If they are equal then that means the bit
    ; wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error

check_long_mode:
    ; test if extended processor info in available
    mov eax, 0x80000000    ; implicit argument for cpuid
    cpuid                  ; get highest supported argument
    cmp eax, 0x80000001    ; it needs to be at least 0x80000001
    jb .no_long_mode       ; if it's less, the CPU is too old for long mode

    ; use extended info to test if long mode is available
    mov eax, 0x80000001    ; argument for extended processor info
    cpuid                  ; returns various feature bits in ecx and edx
    test edx, 1 << 29      ; test if the LM-bit is set in the D-register
    jz .no_long_mode       ; If it's not set, there is no long mode
    ret
.no_long_mode:
    mov al, "2"
    jmp error

set_up_page_tables:
    ; map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11 ; present + writable
    mov [p4_table], eax

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0b11 ; present + writable
    mov [p3_table], eax

    ; map each P2 entry to a huge 2MiB page
    mov ecx, 0         ; counter variable

.map_p2_table:
    ; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
    mov eax, 0x200000  ; 2MiB
    mul ecx            ; start address of ecx-th page
    or eax, 0b10000011 ; present + writable + huge
    mov [p2_table + ecx * 8], eax ; map ecx-th entry

    inc ecx            ; increase counter
    cmp ecx, 512       ; if counter == 512, the whole P2 table is mapped
    jne .map_p2_table  ; else map the next entry

    ; map apic_p2_table into p3_table
    ; bin(0xfec00000) = 0b11 111110111 000000000 000000000000
    ; bin(0xfee00000) = 0b11 111110111 000000000 000000000000
    ; we use entry 0b11 or 3 
    mov eax, apic_p2_table
    or eax, 0b11 ; present + writable
    mov [p3_table + 3*8 ], eax

    ; map ioapic @ 0xfec00000 (which is entry b111110110 or 502 into apic_p2_table) 
    mov eax, 0xfec00000 ; address
    or eax, 0b10000011 ; present + writable + huge
    mov [apic_p2_table + 502 * 8 ], eax ; map ecx-th entry

    ; map lapic @ 0xfee00000 (which is entry b111110111 or 503 into apic_p2_table) 
    mov eax, 0xfee00000 ; address
    or eax, 0b10000011 ; present + writable + huge
    mov [apic_p2_table + 503 * 8 ], eax ; map ecx-th entry

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
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

; Prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64 
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .bss
align 4096

hp4_table:
    resb 4096
hp3_table:
    resb 4096

p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
apic_p2_table:
    resb 4096

stack_bottom:
    resb 4096 * 4096 ; Reserve this many bytes
stack_top:

_bootinfo:
    resb 8 ; Place holder to save bootinfo entry
