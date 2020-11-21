#![no_std]
#![feature(llvm_asm)]
#![feature(global_asm)]
#![feature(thread_local)]

// AB: XXX: this should be a privileged system call 
// at the moment it's marked as unsafe
use libsyscalls::syscalls::{sys_register_cont, sys_discard_cont};
use syscalls::Continuation;

/* 
 * Macro to create a continuation trampoline for the function. 
 *
 * Save all the registers on the stack, then pass the stack frame as 
 * an argument to the Rust register_continuation() function (extern "C" 
 * guarantees the ABI compatibility).
 *
 * Note: we save caller saved registers too as they might be clobbered by 
 * register_cont() function
 *
 * XXX: we cannot simply do "push $foo_err" -- Rust generates a static
 * relocation symbol in this case and the linker complains that it's not 
 * PIE executable. To hack around it we had to call a helper foo_addr 
 * function that returns the address of "foo_err". This makes the linker 
 * happy as somehow it can insert proper PLT relocation symbols for "call" 
 * instructions. 
 */

#[macro_export]
macro_rules! trampoline {
    ($func: ident) => {
    global_asm!(
        core::concat!(r#"
            .text
            .align  16
            "#,
            core::concat!(core::stringify!($func), "_tramp:"),
            r#"
            push %rbx

            push %rcx # scratch

            # cur
            mov %gs:0x0, %rbx
            # end
            mov %gs:0x10, %rcx
            cmp %rcx, %rbx
            jl 1f
            # Continuation stack full
            # FIXME: what should we do?
            hlt
            pop %rcx # scratch
            1:
            pop %rcx # scratch

            # [%rsp] -> saved rbx
            movq %rax, 0x8(%rbx)
            movq %rcx, 0x10(%rbx)
            movq %rdx, 0x18(%rbx)
            movq %rsi, 0x20(%rbx)
            movq %rdi, 0x28(%rbx)
            movq %r8, 0x30(%rbx)
            movq %r9, 0x38(%rbx)
            movq %r10, 0x40(%rbx)

            pushfq
            movq [%rsp], 0x48(%rbx)
            addq $8, %rsp

            movq %r15, 0x50(%rbx)
            movq %r14, 0x58(%rbx)
            movq %r13, 0x60(%rbx)
            movq %r12, 0x68(%rbx)
            movq %r11, 0x70(%rbx)

            # [%rsp] -> saved rbx
            movq [%rsp], 0x78(%rbx)
            movq %rbp, 0x80(%rbx)
            movq %rsp, 0x88(%rbx)

            pushfq
            push %r10
            push %r9
            push %r8
            push %rdi
            push %rsi
            push %rdx
            push %rcx
            push %rax
            call "#, core::concat!(core::stringify!($func), "_addr"),
            r#"

            # func
            mov %rax, 0x48(%rsp)

            # Increment cont stack pointer
            addq $144, %rbx
            mov %rbx, %gs:0x0

            pop %rax
            pop %rcx
            pop %rdx
            pop %rsi
            pop %rdi
            pop %r8
            pop %r9
            pop %r10
            popfq

            # Restore rbx, now stack is fully rewound
            pop %rbx

            jmp "#, core::stringify!($func),
        );
   );
    }
}

/* global_asm!("  
    .text 
    .align  16              
foo_tramp:         
    
    push %rsp
    push %rbp
    push %rbx
    push %r11
    push %r12
    push %r13
    push %r14
    push %r15
    pushfq
    
    push %r10
    push %r9
    push %r8
    push %rdi
    push %rsi
    push %rdx
    push %rcx
    push %rax

    push $foo_err
    
    mov %rsp, %rdi
    call register_cont                       
    subq $144, %rsp                   
    jmp foo ");
*/

#[no_mangle]
pub extern "C" fn register_cont(cont: &Continuation)  {
    unsafe {
        sys_register_cont(cont);
    }
}

#[no_mangle]
pub extern "C" fn discard_cont()  {
    unsafe {
        sys_discard_cont();
    }
}

