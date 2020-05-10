#![no_std]
#![feature(llvm_asm)]
#![feature(global_asm)]

// AB: XXX: this should be a privileged system call 
// at the moment it's marked as unsafe
use libsyscalls::syscalls::sys_register_cont;
use syscalls::Continuation;


/* 
 * Macro to create a continuation trampoline for the function. 
 *
 * Save all the registers on the stack, then pass the stack frame as 
 * an argument to the Rust register_continuation() function (extern "C" 
 * guarantees the ABI compatibility).
 *
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
            push $"#, core::concat!(core::stringify!($func), "_err"),
            r#"
            mov %rsp, %rdi
            call register_cont
            add $8, %rsp
            pop %rax
            pop %rcx
            pop %rdx
            pop %rsi
            pop %rdi
            pop %r8
            pop %r9
            pop %r10
            popfq
            add $64, %rsp
            jmp "#, core::stringify!($func)
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

