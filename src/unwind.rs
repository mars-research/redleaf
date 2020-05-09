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

global_asm!("  
    .text 
    .align  16              
__unwind:
    movq 16(%rdi), %rcx
    movq 24(%rdi), %rdx

    movq 40(%rdi), %rsi
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

    movq 32(%rdi), %rdi

    jmpq *-8(%rsp) ");
    //jmp foo_err ");


extern {
    fn __unwind(cont: &Continuation);
}

pub fn unwind() {
    unsafe {
        println!("Continuation: {:#x?}", CONT);
        __unwind(&CONT);
    }
}

#[no_mangle]
pub fn foo(x: u64, y: u64) {
    unwind();
    println!("you shouldn't see this"); 
}

#[no_mangle]
pub fn foo_err(x: u64, y: u64) {
    println!("foo was aborted, x:{}, y:{}", x, y); 
}

global_asm!("  
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

extern {
    fn foo_tramp(x: u64, y: u64);
}

pub fn unwind_test() {
    unsafe {
        foo_tramp(1, 2);
    }
}

/*
pub fn unwind_test() {

    let rbp: u64;
    let rsp: u64;

    unsafe {
        asm!("mov %rbp, $0" : "=r"(rbp));
    }
    unwind_set_cont(rbp, foo_err as u64);
    foo(); 

    rsp = rbp + core::mem::size_of::<usize>() as u64;
    println!("rbp: {:x}, rsp: {:x}", rbp, rsp); 
}

*/
