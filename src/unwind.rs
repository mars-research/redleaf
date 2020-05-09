//#![feature(asm)]
//#![feature(llvm_asm)]

#[repr(C)]
#[derive(Copy,Clone, Debug)]
pub struct Continuation {
  func: u64,
  /* Caller saved registers (we need them since 
   * function arguments are passed in registers and 
   * we loose them for the restart */
  rax: u64,
  rcx: u64, 
  rdx: u64,
  rsi: u64,
  rdi: u64, 
  r8: u64, 
  r9: u64, 
  r10: u64,

  /* Callee saved registers */
  rflags: u64,
  r15: u64,
  r14: u64,
  r13: u64, 
  r12: u64,
  r11: u64, 
  rbx: u64, 
  rbp: u64,  
  rsp: u64,
}

static mut CONT: Continuation 
    = Continuation { 
        func: 0, rflags: 0, r15: 0, r14: 0, r13: 0, r12: 0, r11: 0, rbx: 0, rbp: 0, rsp:0,
        rax: 0, rcx: 0, rdx: 0, rsi: 0, rdi: 0, r8: 0, r9: 0, r10: 0,
    };

#[no_mangle]
pub extern "C" fn register_cont(cont: &Continuation)  {
    unsafe {
        CONT = *cont;
    }
}

extern {
    fn __unwind(cont: &Continuation);
}

pub fn unwind() {
    unsafe {
        println!("Continuation: {:#x?}", CONT);
        __unwind(&CONT);
    }
}

/* Restore register and stack state right before the invocation
 * make sure that all registers are restored (specifically, caller 
 * registers may be used for passing arguments). Hence we save the 
 * function pointer right below the stack (esp - 8) and jump to 
 * it from there.
 *
 * Note: interrupts are disabled in the kernel, NMIs are handled on a
 * separate IST stack, so nothing should overwrite memory below the 
 * stack (esp - 8).
 *
 * %rdi -- pointer to Continuation
 */

global_asm!("  
    .text 
    .align  16              
__unwind:
    movq 16(%rdi), %rcx
    movq 24(%rdi), %rdx
    movq 32(%rdi), %rsi

    movq 48(%rdi), %r8
    movq 56(%rdi), %r9
    movq 64(%rdi), %r10


    movq 136(%rdi), %rsp
    movq 128(%rdi), %rbp
    movq 120(%rdi), %rbx
    movq 112(%rdi), %r11
    movq 104(%rdi), %r12
    movq 96(%rdi), %r13
    movq 88(%rdi), %r14
    movq 80(%rdi), %r15
    pushq 72(%rdi)
    popfq

    movq (%rdi), %rax
    movq %rax, -8(%rsp)
    movq 8(%rdi), %rax

    movq 40(%rdi), %rdi

    jmpq *-8(%rsp) ");

/* Macro to create a continuation trampoline for the function. 
 *
 * Save all the registers on the stack, then pass the stack frame as 
 * an argument to the Rust register_continuation() function (extern "C" 
 * guarantees the ABI compatibility).
 *
 */
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


/* 
 * Unwind test with simple functions 
 */
#[no_mangle]
pub fn foo(x: u64, y: u64) {
    //unwind();
    println!("you shouldn't see this"); 
}

#[no_mangle]
pub fn foo_err(x: u64, y: u64) {
    println!("foo was aborted, x:{}, y:{}", x, y); 
}

extern {
    fn foo_tramp(x: u64, y: u64);
}

trampoline!(foo);

/*
 * Unwind test with traits
 */

pub trait FooTrait {
    fn simple_result(&self, x: u64) -> Result<u64, i64>;
}

pub struct Foo {
    id: u64,
}

impl FooTrait for Foo {
    fn simple_result(&self, x: u64) -> Result<u64, i64> {
        let r = self.id; 
        unwind();
        Ok(r)
    }
}

static FOO: Foo = Foo {id: 55};

#[no_mangle]
pub extern fn simple_result(s: &Foo, x: u64) -> Result<u64, i64> {
    println!("simple_result: s.id:{}, x:{}", s.id, x);
    let r = s.simple_result(x);
    println!("simple_result: you shouldn't see this");
    r
}

#[no_mangle]
pub extern fn simple_result_err(s: &Foo, x: u64) -> Result<u64, i64> {
    println!("simple_result was aborted, s.id:{}, x:{}", s.id, x);
    Err(-1)
}

extern {
    fn simple_result_tramp(s:&Foo, x: u64) -> Result<u64, i64>;
}

trampoline!(simple_result);

pub fn unwind_test() {
    unsafe {
        foo_tramp(1, 2);
        let r = simple_result_tramp(&FOO, 3); 
        match r {
            Ok(n)  => println!("simple_result (ok):{}", n),
            Err(e) => println!("simple_result, err: {}", e),
        }

    }
}


