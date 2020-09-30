global switch

; Context (each field is 8 bytes)
;  0x0: r15
;  0x8: r14
; 0x10: r13
; 0x18: r12
; 0x20: r11
; 0x28: rbx
; 0x30: rbp
; 0x38: rsp
; 0x40: rflags

; switch_reg reg, offset
;
; Saves reg to Context at rdi, loads reg from Context at rsi
%macro switch_reg 2
	mov [rdi + %2], %1
	mov %1, [rsi + %2]
%endmacro

; switch(prev_ctx: *Context, next_ctx: *Context)
;
; Restores the next context
switch:
	; prev_ctx: rdi
	; next_ctx: rsi

	switch_reg r15, 0x0
	switch_reg r14, 0x8
	switch_reg r13, 0x10
	switch_reg r12, 0x18
	switch_reg rbx, 0x28
	switch_reg rbp, 0x30
	switch_reg rsp, 0x38

	; rflags
	pushfq
	pop r9
	switch_reg r9, 0x40
	push r9
	popfq

    ret
