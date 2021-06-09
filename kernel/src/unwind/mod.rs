//! RedLeaf unwind runtime.
//!
//! Unwinds across domains using DWARF information.
//! This implementation is based upon Theseus's unwinder as well
//! as gimli's unwind-rs example.
//!
//! In our design, we obtain

use core::fmt;
use alloc::boxed::Box;

use fallible_iterator::FallibleIterator;
use gimli::{
    UnwindSection, 
    UnwindTableRow, 
    EhFrame, 
    BaseAddresses, 
    UninitializedUnwindContext, 
    FrameDescriptionEntry,
    Pointer,
    EndianSlice,
    NativeEndian,
    CfaRule,
    RegisterRule,
    X86_64
};
use x86::bits64::paging::VAddr;

use crate::domain::Domain;

use syscalls::UnwindCause;

mod registers;
use registers::{LandingRegisters, Registers, SavedRegs};

mod lsda;

type NativeEndianSliceReader<'i> = EndianSlice<'i, NativeEndian>;

/// Dereferences a `Pointer` type found in unwinding information,
/// which is either direct (no dereference) or indirect. 
/// Doing so is unsafe because the value of the `Pointer` is not checked. 
unsafe fn deref_ptr(ptr: Pointer) -> u64 {
    match ptr {
        Pointer::Direct(x) => x,
        Pointer::Indirect(x) => *(x as *const u64),
    }
}

/// Returns the current kernel's .text
fn text_start() -> u64 {
    extern "C" {
        static __text_start: u8;
    }
    unsafe { &__text_start as *const _ as u64 }
}

pub struct UnwindingContext {
    /// The iterator over the current call stack, in which the "next" item in the iterator
    /// is the previous frame in the call stack (the caller frame).
    stack_frame_iter: StackFrameIter,

    /// The cause of the unwind.
    cause: Option<UnwindCause>,

    /// The domain that initiated the unwind.
    initiator: Option<Domain>,
}

#[derive(Debug)]
struct UnwindRowReference {
    caller: u64,
    domain: Option<Domain>,
}

impl UnwindRowReference {
    /// Create a UnwindRowReference for a call site in a domain.
    fn from_domain(caller: u64, domain: Domain) -> Option<Self> {
        if domain.get_section_slice(".eh_frame").is_some() {
            let row_ref = Self {
                caller,
                domain: Some(domain),
            };

            Some(row_ref)
        } else {
            None
        }
    }

    /// Create a UnwindRowReference for a call site (possibly) in the kernel.
    fn from_kernel(caller: u64) -> Self {
        Self {
            caller,
            domain: None,
        }
    }

    fn with_unwind_info<O, F>(&self, mut f: F) -> Result<O, &'static str>
        where F: FnMut(&FrameDescriptionEntry<NativeEndianSliceReader, usize>, &UnwindTableRow<NativeEndianSliceReader>) -> Result<O, &'static str>
    {
        let (eh_frame, base_addrs) = match &self.domain {
            Some(domain) => {
                let slice = domain.get_section_slice(".eh_frame").unwrap();
                let eh_frame_offset = slice as *const _ as *const u8 as u64;
                let text_offset = domain.get_section_slice(".text").unwrap()
                    as *const _ as *const u8 as u64;

                let eh_frame = EhFrame::new(slice, NativeEndian);
                let base_addrs = BaseAddresses::default()
                    .set_eh_frame(eh_frame_offset)
                    .set_text(text_offset);

                (eh_frame, base_addrs)
            }
            None => {
                let kernel_elf = crate::panic::get_kernel_elf().r#try()
                    .expect("Kernel unwinding hasn't been set up yet");

                let file = &kernel_elf.file;
                let text = file.find_section_by_name(".text").unwrap()
                    .raw_data(file);
                let eh_frame = file.find_section_by_name(".eh_frame")
                    .expect(".eh_frame is not present in kernel ELF")
                    .raw_data(file);

                let eh_frame_offset = eh_frame as *const _ as *const u8 as u64;

                let eh_frame = EhFrame::new(eh_frame, NativeEndian);
                let base_addrs = BaseAddresses::default()
                    .set_eh_frame(eh_frame_offset)
                    .set_text(text_start());

                (eh_frame, base_addrs)
            }
        };

        let mut unwind_ctx = UninitializedUnwindContext::new();
        let fde = eh_frame.fde_for_address(&base_addrs, self.caller, EhFrame::cie_from_offset).map_err(|_e| {
            log::error!("gimli error: {:?}", _e);
            "gimli error while finding FDE for address"
        })?;
        let unwind_table_row = fde.unwind_info_for_address(&eh_frame, &base_addrs, &mut unwind_ctx, self.caller).map_err(|_e| {
            log::error!("gimli error: {:?}", _e);
            "gimli error while finding unwind info for address"
        })?;
        
        f(&fde, &unwind_table_row)
    }
}

/// A single frame in the stack, which contains
/// unwinding-related information for a single function call's stack frame.
/// 
/// See each method for documentation about the members of this struct.
#[derive(Debug)]
pub struct StackFrame {
    personality: Option<u64>,
    lsda: Option<u64>,
    initial_address: u64,
    call_site_address: u64,
}

impl StackFrame {
    /// The address of the personality function that corresponds
    /// to this stack frame's unwinding routine, if needed for this stack frame.
    /// In Rust, this is always the same function, the one defined as the `eh_personality`
    /// language item, something required by the compiler.
    /// 
    /// Note that in Theseus we do not use a personality function,
    /// as we use a custom unwinding flow that bypasses invoking the personality function.
    pub fn personality(&self) -> Option<u64> {
        self.personality
    }

    /// The address of the Language-Specific Data Area (LSDA)
    /// that is needed to discover the unwinding cleanup routines (landing pads)
    /// for this stack frame. 
    /// Typically, this points to an area within the `.gcc_except_table` section,
    /// which then needs to be parsed.
    pub fn lsda(&self) -> Option<u64> {
        self.lsda
    }

    /// The *call site* of this stack frame, i.e.,
    /// the address of the instruction that called the next function in the call stack.
    pub fn call_site_address(&self) -> u64 {
        self.call_site_address
    }

    /// The address (starting instruction pointer) of the function
    /// corresponding to this stack frame. 
    /// This points to the top (entry point) of that function.
    pub fn initial_address(&self) -> u64 {
        self.initial_address
    }
}

/// An iterator over the stack frames on the current task's call stack,
/// which works in reverse calling order from the current function
/// up the call stack to the very first function on the stack,
/// at which point it will return `None`. 
/// 
/// This is a lazy iterator: the previous frame in the call stack
/// is only calculated upon invocation of the `next()` method. 
/// 
/// This can be used with the `FallibleIterator` trait.
pub struct StackFrameIter {
    /// The values of the registers that exited during the stack frame
    /// that is currently being iterated over. 
    /// 
    /// These register values will change on each invocation of `next()`
    /// as different stack frames are successively iterated over.
    registers: Registers,
    /// Unwinding state related to the previous frame in the call stack:
    /// a reference to its row/entry in the unwinding table,
    /// and the Canonical Frame Address (CFA value) that is used to determine the next frame.
    state: Option<(UnwindRowReference, u64)>,
    /// An extra offset that is used to adjust the calculation of the CFA in certain circumstances, 
    /// primarily when unwinding through an exception/interrupt handler stack frame `B` 
    /// to a frame `A` that caused the exception, even though frame `A` did not "call" frame `B`.
    /// This will have a different value based on what the CPU did, e.g., pushed error codes onto the stack. 
    /// 
    /// If `Some`, the latest stack frame produced by this iterator was an exception handler stack frame.
    cfa_adjustment: Option<i64>,
    /// This is set to true when the previous stack frame was an exception/interrupt handler,
    /// which is useful in the case of taking into account the CPU pushing an `ExceptionStackFrame` onto the stack.
    /// The DWARF debugging/unwinding info cannot account for this because an interrupt or exception happening 
    /// is not the same as a regular function "call" happening.
    last_frame_was_exception_handler: bool,
}

impl fmt::Debug for StackFrameIter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "StackFrameIter {{\nRegisters: {:?},\nstate: {:#X?}\n}}", self.registers, self.state)
        write!(f, "StackFrameIter {{\nRegisters: {:?}\n}}", self.registers)
    }
}

impl StackFrameIter {
    /// Create a new iterator over stack frames that starts from the current frame
    /// and uses the given `Registers` values as a starting point. 
    /// 
    /// Note: ideally, this shouldn't be public since it needs to be invoked with the correct initial register values.
    #[doc(hidden)]
    pub fn new(registers: Registers) -> Self {
        StackFrameIter {
            registers,
            state: None,
            cfa_adjustment: None,
            last_frame_was_exception_handler: false,
        }
    }

    /// Returns the array of register values as they existed during the stack frame
    /// that is currently being iterated over. 
    /// 
    /// After the [`next()`](#method.next.html) is invoked to iterate to a given stack frame,
    /// this function will return the register values for that frame that was just iterated to. 
    /// Successive calls to this function will keep returning the same register values 
    /// until the `next()` method is invoked again. 
    /// 
    /// This is necessary in order to restore the proper register values 
    /// before jumping to the **landing pad** (a cleanup function or exception/panic catcher)
    /// such that the landing pad function will actually execute properly with the right context.
    pub fn registers(&self) -> &Registers {
        &self.registers
    }
}

impl FallibleIterator for StackFrameIter {
    type Item = StackFrame;
    type Error = &'static str;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let registers = &mut self.registers;
        let prev_cfa_adjustment = self.cfa_adjustment;

        if let Some((unwind_row_ref, cfa)) = self.state.take() {
            let mut newregs = registers.clone();
            newregs[X86_64::RA] = None;

            // For x86_64, the stack pointer is defined to be the previously-calculated CFA.
            newregs[X86_64::RSP] = Some(cfa);
            // If this frame is an exception/interrupt handler, we need to adjust RSP and the return address RA accordingly.
            if let Some(extra_offset) = prev_cfa_adjustment {
                newregs[X86_64::RSP] = Some(cfa.wrapping_add(extra_offset as u64));
                #[cfg(not(downtime_eval))]
                log::trace!("adjusting RSP to {:X?}", newregs[X86_64::RSP]);
            } 

            unwind_row_ref.with_unwind_info(|_fde, row| {
                // There is some strange behavior when moving up the call stack 
                // from an exception handler function's frame `B` to a frame `A` that resulted in the exception,
                // since frame `A` did not call frame `B` directly, 
                // and since the CPU may have pushed an error code onto the stack,
                // which messes up the DWARF info that calculates register values properly. 
                //
                // In this case, the `cfa` value must be modified to account for that error code 
                // being pushed onto the stack by adding `8` (the error code's size in bytes) to the `cfa` value.
                //
                // Also, the return address (RA) must be calculated differently, not using the below register rules.
                for &(reg_num, ref rule) in row.registers() {
                    // debug!("Looking at register rule:  {:?} {:?}", reg_num, rule);
                    // The stack pointer (RSP) is given by the CFA calculated during the previous iteration;
                    // there should *not* be a register rule defining the value of the RSP directly.
                    if reg_num == X86_64::RSP {
                        #[cfg(not(downtime_eval))]
                        log::warn!("Ignoring unwind row's register rule for RSP {:?}, which is invalid on x86_64 because RSP is always set to the CFA value.", rule);
                        continue;
                    }

                    // If this stack frame is an exception handler, the return address wouldn't have been pushed onto the stack as with normal call instructions.
                    // Instead, it would've been pushed onto the stack by the CPU as part of the ExceptionStackFrame, so we have to look for it there.
                    //
                    // We know that the stack currently looks like this:
                    // |-- address --|------------  Item on the stack -------------|
                    // |  <lower>    |  ...                                        |
                    // | CFA         |  error code                                 |
                    // | CFA + 0x08  |  Exception stack frame: instruction pointer |
                    // | CFA + 0x10  |                         code segment        |
                    // | CFA + 0x18  |                         cpu flags           |
                    // | CFA + 0x20  |                         stack pointer       |
                    // | CFA + 0x28  |                         stack segment       |
                    // |  <higher>   |  ...                                        |
                    // |-------------|---------------------------------------------|
                    //
                    // Thus, we want to skip the error code so we can get the instruction pointer, 
                    // i.e., the value at CFA + 0x08.
                    if reg_num == X86_64::RA {
                        if let Some(_) = prev_cfa_adjustment {
                            let size_of_error_code = core::mem::size_of::<usize>();
                            // TODO FIXME: only skip the error code if the prev_cfa_adjustment included it
                            let value = unsafe { *(cfa.wrapping_add(size_of_error_code as u64) as *const u64) };
                            #[cfg(not(downtime_eval))]
                            log::trace!("Using return address from CPU-pushed exception stack frame. Value: {:#X}", value);
                            newregs[X86_64::RA] = Some(value);
                            continue;
                        }
                    }

                    newregs[reg_num] = match *rule {
                        RegisterRule::Undefined => return Err("StackFrameIter: encountered an unsupported RegisterRule::Undefined"), // registers[reg_num],
                        RegisterRule::SameValue => registers[reg_num],
                        RegisterRule::Register(other_reg_num) => registers[other_reg_num],
                        // This is the most common register rule (in fact, the only one we've seen),
                        // so we may have to adapt the logic herein for use in other rules. 
                        RegisterRule::Offset(offset) => {
                            let value = unsafe { *(cfa.wrapping_add(offset as u64) as *const u64) };
                            // trace!("     cfa: {:#X}, addr: {:#X}, value: {:#X}", cfa, cfa.wrapping_add(offset as u64), value);
                            Some(value)
                        }
                        RegisterRule::ValOffset(offset) => Some(cfa.wrapping_add(offset as u64)),
                        RegisterRule::Expression(_) => return Err("StackFrameIter: encountered an unsupported RegisterRule::Expression"),
                        RegisterRule::ValExpression(_) => return Err("StackFrameIter: encountered an unsupported RegisterRule::ValExpression"),
                        RegisterRule::Architectural => return Err("StackFrameIter: encountered an unsupported RegisterRule::Architectural"),
                    };
                }
                Ok(())
            })?;

            *registers = newregs;
        }

        // The return address (used to find the caller's stack frame) should be in the newly-calculated register set.
        // If there isn't one, or if it's 0, then we have reached the beginning of the call stack, and are done iterating.
        let return_address = match registers[X86_64::RA] {
            Some(0) | None => return Ok(None),
            Some(ra) => ra,
        };
        
        // The return address (RA register) actually points to the *next* instruction (1 byte past the call instruction),
        // because the processor has advanced it to continue executing after the function returns.
        // As x86 has variable-length instructions, we don't know exactly where the previous instruction starts,
        // but we know that subtracting `1` will give us an address *within* that previous instruction.
        let caller = return_address - 1;
        // TODO FIXME: only subtract 1 for non-"fault" exceptions, e.g., page faults should NOT subtract 1
        // trace!("call_site_address: {:#X}", caller);


        // Get unwind info for the call site address
        let row_ref = match crate::domain::find_domain_containing(VAddr::from(caller)) {
            Some(domain) => {
                UnwindRowReference::from_domain(caller, domain)
                    .ok_or("couldn't get eh_frame section in caller's containing domain")?
            }
            None => {
                log::warn!("Call site 0x{:x?} seems to be in the kernel", caller);
                UnwindRowReference::from_kernel(caller)
            }
        };

        let mut cfa_adjustment: Option<i64> = None;
        let mut this_frame_is_exception_handler = false;

        let (cfa, frame) = row_ref.with_unwind_info(|fde, row| {
            // trace!("ok: {:?} (0x{:x} - 0x{:x})", row.cfa(), row.start_address(), row.end_address());
            let cfa = match *row.cfa() {
                CfaRule::RegisterAndOffset{register, offset} => {
                    // debug!("CfaRule:RegisterAndOffset: reg {:?}, offset: {:#X}", register, offset);
                    let reg_value = registers[register].ok_or_else(|| {
                        #[cfg(not(downtime_eval))]
                        log::error!("CFA rule specified register {:?} with offset {:#X}, but register {:?}({}) had no value!", register, offset, register, register.0);
                        "CFA rule specified register with offset, but that register had no value."
                    })?;
                    reg_value.wrapping_add(offset as u64)
                }
                CfaRule::Expression(_expr) => {
                    #[cfg(not(downtime_eval))]
                    log::error!("CFA rules based on Expressions are not yet supported. Expression: {:?}", _expr);
                    return Err("CFA rules based on Expressions are not yet supported.");
                }
            };
            
            // trace!("initial_address: {:#X}", fde.initial_address());

            // If the next stack frame is an exception handler, then the CPU pushed an `ExceptionStackFrame`
            // onto the stack, completely unbeknownst to the DWARF debug info. 
            // Thus, we need to adjust this next frame's stack pointer (i.e., `cfa` which becomes the stack pointer)
            // to account for the change in stack contents. 
            // TODO FIXME: check for any type of exception/interrupt handler, and differentiate between error codes

            /*
            cfa_adjustment = if interrupts::is_exception_handler_with_error_code(fde.initial_address()) {
                let size_of_error_code: i64 = core::mem::size_of::<usize>() as i64;
                #[cfg(not(downtime_eval))]
                log::trace!("StackFrameIter: next stack frame has a CPU-pushed error code on the stack, adjusting CFA to {:#X}", cfa);

                // TODO: we need to set this to true for any exception/interrupt handler, not just those with error codes.
                // If there is an error code pushed, then we need to account for that additionally beyond the exception stack frame being pushed.
                let size_of_exception_stack_frame: i64 = 5 * 8;
                #[cfg(not(downtime_eval))]
                log::trace!("StackFrameIter: next stack frame is an exception handler: adding {:#X} to cfa, new cfa: {:#X}", size_of_exception_stack_frame, cfa);
                
                this_frame_is_exception_handler = true;
                Some(size_of_error_code + size_of_exception_stack_frame)
            } else {
                None
            };
            */

            // trace!("cfa is {:#X}", cfa);

            let frame = StackFrame {
                personality: None, // we don't use the personality function in Theseus
                lsda: fde.lsda().map(|x| unsafe { deref_ptr(x) }),
                initial_address: fde.initial_address(),
                call_site_address: caller,
            };
            Ok((cfa, frame))
        })?;

        // since we can't double-borrow `self` mutably in the above closure, we assign its state(s) here.
        self.cfa_adjustment = cfa_adjustment;
        self.last_frame_was_exception_handler = this_frame_is_exception_handler;
        self.state = Some((row_ref, cfa));

        // return the stack frame that we just iterated to
        Ok(Some(frame))
    }
}

pub trait FuncWithRegisters = Fn(Registers) -> Result<(), &'static str>;
type RefFuncWithRegisters<'a> = &'a dyn FuncWithRegisters;


/// This function saves the current CPU register values onto the stack (to preserve them)
/// and then invokes the given closure with those registers as the argument.
/// 
/// In general, this is useful for jumpstarting the unwinding procedure,
/// since we have to start from the current call frame and work backwards up the call stack 
/// while applying the rules for register value changes in each call frame
/// in order to arrive at the proper register values for a prior call frame.
pub fn invoke_with_current_registers<F>(f: F) -> Result<(), &'static str> 
    where F: FuncWithRegisters 
{
    let f: RefFuncWithRegisters = &f;
    let result = unsafe { 
        let res_ptr = unwind_trampoline(&f);
        let res_boxed = Box::from_raw(res_ptr);
        *res_boxed
    };
    return result;
    // this is the end of the code in this function, the following is just inner functions.


    /// This is an internal assembly function used by `invoke_with_current_registers()` 
    /// that saves the current register values by pushing them onto the stack
    /// before invoking the function "unwind_recorder" with those register values as the only argument.
    /// This is needed because the unwind info tables describe register values as operations (offsets/addends)
    /// that are relative to the current register values, so we must have those current values as a starting point.
    /// 
    /// The argument is a pointer to a function reference, so effectively a pointer to a pointer. 
    #[naked]
    #[inline(never)]
    unsafe extern "C" fn unwind_trampoline(_func: *const RefFuncWithRegisters) -> *mut Result<(), &'static str> {
        // This is a naked function, so you CANNOT place anything here before the asm block, not even log statements.
        // This is because we rely on the value of registers to stay the same as whatever the caller set them to.
        // DO NOT touch RDI register, which has the `_func` function; it needs to be passed into unwind_recorder.
        llvm_asm!("
            # copy the stack pointer to RSI
            movq %rsp, %rsi
            pushq %rbp
            pushq %rbx
            pushq %r12
            pushq %r13
            pushq %r14
            pushq %r15
            # To invoke `unwind_recorder`, we need to put: 
            # (1) the func in RDI (it's already there, just don't overwrite it),
            # (2) the stack in RSI,
            # (3) a pointer to the saved registers in RDX.
            movq %rsp, %rdx   # pointer to saved regs (on the stack)
            call unwind_recorder
            # restore saved registers
            popq %r15
            popq %r14
            popq %r13
            popq %r12
            popq %rbx
            popq %rbp
            ret
        ");
        core::hint::unreachable_unchecked();
    }


    /// The calling convention dictates the following order of arguments: 
    /// * first arg in `RDI` register, the function (or closure) to invoke with the saved registers arg,
    /// * second arg in `RSI` register, the stack pointer,
    /// * third arg in `RDX` register, the saved register values used to recover execution context
    ///   after we change the register values during unwinding,
    #[no_mangle]
    unsafe extern "C" fn unwind_recorder(
        func: *const RefFuncWithRegisters,
        stack: u64,
        saved_regs: *mut SavedRegs,
    ) -> *mut Result<(), &'static str> {
        let func = &*func;
        let saved_regs = &*saved_regs;

        let mut registers = Registers::default();
        registers[X86_64::RBX] = Some(saved_regs.rbx);
        registers[X86_64::RBP] = Some(saved_regs.rbp);
        registers[X86_64::RSP] = Some(stack + 8); // the stack value passed in is one pointer width before the real RSP
        registers[X86_64::R12] = Some(saved_regs.r12);
        registers[X86_64::R13] = Some(saved_regs.r13);
        registers[X86_64::R14] = Some(saved_regs.r14);
        registers[X86_64::R15] = Some(saved_regs.r15);
        registers[X86_64::RA]  = Some(*(stack as *const u64));

        let res = func(registers);
        Box::into_raw(Box::new(res))
    }
}

pub fn start_unwinding(cause: Option<UnwindCause>, stack_frames_to_skip: usize) -> Result<(), &'static str> {
    // Here we have to be careful to have no resources waiting to be dropped/freed/released on the stack. 
    let unwinding_context_ptr = {
        Box::into_raw(Box::new(
            UnwindingContext {
                stack_frame_iter: StackFrameIter::new(
                    // we will set the real register values later, in the `invoke_with_current_registers()` closure.
                    Registers::default()
                ), 
                cause,
                initiator: None, // FIXME
            }
        ))
    };

    // IMPORTANT NOTE!!!!
    // From this point on, if there is a failure, we need to free the unwinding context pointer to avoid leaking things.


    // We pass a pointer to the unwinding context to this closure. 
    let res = invoke_with_current_registers(|registers| {
        // set the proper register values before we used the 
        {  
            // SAFE: we just created this pointer above
            let unwinding_context = unsafe { &mut *unwinding_context_ptr };
            unwinding_context.stack_frame_iter.registers = registers;
            
            // Skip the first several frames, e.g., to skip unwinding the panic entry point functions themselves.
            for _i in 0..stack_frames_to_skip {
                unwinding_context.stack_frame_iter.next()
                    .map_err(|_e| {
                        log::error!("error skipping call stack frame {} in unwinder", _i);
                        "error skipping call stack frame in unwinder"
                    })?
                    .ok_or("call stack frame did not exist (we were trying to skip it)")?;
            }
        }

        continue_unwinding(unwinding_context_ptr)
    });

    match &res {
        &Ok(()) => {
            log::debug!("unwinding procedure has reached the end of the stack.");
        }
        &Err(e) => {
            log::error!("BUG: unwinding the first stack frame returned unexpectedly. Error: {}", e);
        }
    }
    cleanup_unwinding_context(unwinding_context_ptr);
}


/// Continues the unwinding process from the point it left off at, 
/// which is defined by the given unwinding context.
/// 
/// This returns an error upon failure, 
/// and an `Ok(())` when it reaches the end of the stack and there are no more frames to unwind.
/// When either value is returned (upon a return of any kind),
/// **the caller is responsible for cleaning up** the given `UnwindingContext`.
/// 
/// Upon successfully continuing to iterate up the call stack, this function will actually not return at all. 
fn continue_unwinding(unwinding_context_ptr: *mut UnwindingContext) -> Result<(), &'static str> {
    let stack_frame_iter = unsafe { &mut (*unwinding_context_ptr).stack_frame_iter };
    
    #[cfg(not(downtime_eval))]
    log::trace!("continue_unwinding(): stack_frame_iter: {:#X?}", stack_frame_iter);
    
    let (mut regs, landing_pad_address) = if let Some(frame) = stack_frame_iter.next().map_err(|e| {
        log::error!("continue_unwinding: error getting next stack frame in the call stack: {}", e);
        "continue_unwinding: error getting next stack frame in the call stack"
    })? {
        /* Theseus
        #[cfg(not(downtime_eval))] {
            log::info!("Unwinding StackFrame: {:#X?}", frame);
            log::info!("  In func: {:?}", stack_frame_iter.namespace().get_section_containing_address(VirtualAddress::new_canonical(frame.initial_address() as usize), false));
            log::info!("  Regs: {:?}", stack_frame_iter.registers());
        }
        */

        if let Some(lsda) = frame.lsda() {
            let lsda = VAddr::from(lsda);
            if let Some((domain, section_name)) = crate::domain::find_section_containing(lsda) {
                log::info!("  parsing LSDA section from {:?}", domain);

                let lsda_slice = domain.get_section_slice(&section_name)
                    .expect("continue_unwinding(): the section just went poof??");
                
                let table = lsda::GccExceptTableArea::new(lsda_slice, NativeEndian, frame.initial_address());

                // {
                //     let mut iter = table.call_site_table_entries().map_err(|_| "BAD TABLE")?;
                //     while let Some(entry) = iter.next().map_err(|_| "BAD ITER")? {
                //         debug!("    {:#X?}", entry);
                //     }
                // }

                let entry = match table.call_site_table_entry_for_address(frame.call_site_address()) {
                    Ok(x) => x,
                    Err(e) => {
                        #[cfg(not(downtime_eval))]
                        log::error!("continue_unwinding(): couldn't find a call site table entry for this stack frame's call site address {:#X}. Error: {}", frame.call_site_address(), e);
                        
                        // Now we don't have an exact match. We try to use the previous
                        let mut iter = table.call_site_table_entries().map_err(|_e| {"Couldn't find call_site_table_entries"})?;

                        let mut closest_entry = None;
                        while let Some(entry) = iter.next().map_err(|_e| {"Couldn't iterate through the entries"})? {
                            if entry.range_of_covered_addresses().start < frame.call_site_address() {
                                closest_entry =  Some(entry);
                            }
                        }
                        
                        if let Some (closest_entry) = closest_entry {
                            #[cfg(not(downtime_eval))]
                            log::debug!("No unwind info for address. Using the closeset");
                            closest_entry
                        } else {
                            return Err("continue_unwinding(): couldn't find a call site table entry for this stack frame's call site address.");
                        }
                    }
                };

                #[cfg(not(downtime_eval))]
                log::debug!("Found call site entry for address {:#X}: {:#X?}", frame.call_site_address(), entry);
                (stack_frame_iter.registers().clone(), entry.landing_pad_address())
            } else {
                log::error!("  BUG: couldn't find LSDA section (.gcc_except_table) for LSDA address: {:#X}", lsda);
                return Err("BUG: couldn't find LSDA section (.gcc_except_table) for LSDA address specified in stack frame");
            }
        } else {
            #[cfg(not(downtime_eval))]
            log::trace!("continue_unwinding(): stack frame has no LSDA");
            return continue_unwinding(unwinding_context_ptr);
        }
    } else {
        #[cfg(not(downtime_eval))]
        log::trace!("continue_unwinding(): NO REMAINING STACK FRAMES");
        return Ok(());
    };

    // Even if this frame has LSDA, it may still not have a landing pad function.
    let landing_pad_address = match landing_pad_address {
        Some(lpa) => lpa,
        _ => {
            #[cfg(not(downtime_eval))]
            log::warn!("continue_unwinding(): stack frame has LSDA but no landing pad");
            return continue_unwinding(unwinding_context_ptr);
        }
    };

    // Exception/interrupt handlers appear to have no real cleanup routines, despite having an LSDA entry. 
    // Thus, we skip unwinding an exception handler frame because its landing pad will point to an invalid instruction (usually `ud2`).
    if stack_frame_iter.last_frame_was_exception_handler {
        let landing_pad_value: u16 = unsafe { *(landing_pad_address as *const u16) };
        #[cfg(not(downtime_eval))]
        log::warn!("Skipping exception/interrupt handler's landing pad (cleanup function) at {:#X}, which points to {:#X} (UD2: {})", 
            landing_pad_address, landing_pad_value, landing_pad_value == 0x0B0F,  // the `ud2` instruction
        );
        return continue_unwinding(unwinding_context_ptr);
    }

    // Jump to the actual landing pad function, or rather, a function that will jump there after setting up register values properly.
    #[cfg(not(downtime_eval))]
    log::debug!("Jumping to landing pad (cleanup function) at {:#X}", landing_pad_address);
    // Once the unwinding cleanup function is done, it will call _Unwind_Resume (technically, it jumps to it),
    // and pass the value in the landing registers' RAX register as the argument to _Unwind_Resume. 
    // So, whatever we put into RAX in the landing regs will be placed into the first arg (RDI) in _Unwind_Resume.
    // This is arch-specific; for x86_64 the transfer is from RAX -> RDI, for ARM/AARCH64, the transfer is from R0 -> R1 or X0 -> X1.
    // See this for more mappings: <https://github.com/rust-lang/rust/blob/master/src/libpanic_unwind/gcc.rs#L102>
    regs[gimli::X86_64::RAX] = Some(unwinding_context_ptr as u64);
    unsafe {
        land(&regs, landing_pad_address)?;
    }
    log::error!("BUG: call to unwind::land() returned, which should never happen!");
    Err("BUG: call to unwind::land() returned, which should never happen!")
}

/// This function is invoked after each unwinding cleanup routine has finished.
/// Thus, this is a middle point in the unwinding execution flow; 
/// here we need to continue (*resume*) the unwinding procedure 
/// by basically figuring out where we just came from and picking up where we left off. 
#[doc(hidden)]
pub fn unwind_resume(unwinding_context_ptr: usize) -> ! {
    // trace!("unwind_resume(): unwinding_context_ptr value: {:#X}", unwinding_context_ptr);
    let unwinding_context_ptr = unwinding_context_ptr as *mut UnwindingContext;

    match continue_unwinding(unwinding_context_ptr) {
        Ok(()) => {
            #[cfg(not(downtime_eval))]
            log::debug!("unwind_resume(): continue_unwinding() returned Ok(), meaning it's at the end of the call stack.");
        }
        Err(e) => {
            log::error!("BUG: in unwind_resume(): continue_unwinding() returned an error: {}", e);
        }
    }
    // here, cleanup the unwinding state and kill the task
    cleanup_unwinding_context(unwinding_context_ptr);
}

// For domains, this is implemented by libredleaf
#[no_mangle]
extern "C" fn _Unwind_Resume(arg: usize) -> ! {
    unwind_resume(arg)
}

/// This function should be invoked when the unwinding procedure is finished, or cannot be continued any further.
/// It cleans up the `UnwindingContext` object pointed to by the given pointer and marks the current task as killed.
fn cleanup_unwinding_context(unwinding_context_ptr: *mut UnwindingContext) -> ! {
    // Recover ownership of the unwinding context from its pointer
    let unwinding_context_boxed = unsafe { Box::from_raw(unwinding_context_ptr) };
    let unwinding_context = *unwinding_context_boxed;

    drop(unwinding_context.stack_frame_iter);
    // FIXME: Correctly clean up task
    log::error!("XXX Unwind completed/failed");
    log::error!("TODO: cleanup");
    loop {}

    // let (stack_frame_iter, cause, current_task) = unwinding_context.into();
    // drop(stack_frame_iter);

    /* Theseus
    let failure_cleanup_function = {
        let t = current_task.lock();
        t.failure_cleanup_function.clone()
    };
    #[cfg(not(downtime_eval))]
    log::warn!("cleanup_unwinding_context(): invoking the task_cleanup_failure function for task {:?}", current_task);
    failure_cleanup_function(current_task, cause)
    */
}

/// **Landing** refers to the process of jumping to a handler for a stack frame,
/// e.g., an unwinding cleanup function, or an exception "catch" block.
/// 
/// This function basically fills the actual CPU registers with the values in the given `LandingRegisters`
/// and then jumps to the exception handler (landing pad) pointed to by the stack pointer (RSP) in those `LandingRegisters`.
/// 
/// This is similar in design to how the latter half of a context switch routine
/// must restore the previously-saved registers for the next task.
unsafe fn land(regs: &Registers, landing_pad_address: u64) -> Result<(), &'static str> {
    let mut landing_regs = LandingRegisters {
        rax: regs[X86_64::RAX].unwrap_or(0),
        rbx: regs[X86_64::RBX].unwrap_or(0),
        rcx: regs[X86_64::RCX].unwrap_or(0),
        rdx: regs[X86_64::RDX].unwrap_or(0),
        rdi: regs[X86_64::RDI].unwrap_or(0),
        rsi: regs[X86_64::RSI].unwrap_or(0),
        rbp: regs[X86_64::RBP].unwrap_or(0),
        r8:  regs[X86_64::R8 ].unwrap_or(0),
        r9:  regs[X86_64::R9 ].unwrap_or(0),
        r10: regs[X86_64::R10].unwrap_or(0),
        r11: regs[X86_64::R11].unwrap_or(0),
        r12: regs[X86_64::R12].unwrap_or(0),
        r13: regs[X86_64::R13].unwrap_or(0),
        r14: regs[X86_64::R14].unwrap_or(0),
        r15: regs[X86_64::R15].unwrap_or(0),
        rsp: regs[X86_64::RSP].ok_or("unwind::land(): RSP was None, \
            it must be set so that the landing pad function can execute properly."
        )?,
    };

    // Now place the landing pad function's address at the "bottom" of the stack
    // -- not really the bottom of the whole stack, just the last thing to be popped off after the landing_regs.
    landing_regs.rsp -= core::mem::size_of::<u64>() as u64;
    *(landing_regs.rsp as *mut u64) = landing_pad_address;
    // trace!("unwind_lander regs: {:#X?}", landing_regs);
    unwind_lander(&landing_regs);
    // this is the end of the code in this function, the following is just inner functions.


    /// This function places the values of the given landing registers
    /// into the actual CPU registers, and then jumps to the landing pad address
    /// specified by the stack pointer in those registers. 
    /// 
    /// It is marked as divergent (returning `!`) because it doesn't return to the caller,
    /// instead it returns (jumps to) that landing pad address.
    #[naked]
    #[inline(never)]
    unsafe extern fn unwind_lander(_regs: *const LandingRegisters) -> !{
        llvm_asm!("
            movq %rdi, %rsp
            popq %rax
            popq %rbx
            popq %rcx
            popq %rdx
            popq %rdi
            popq %rsi
            popq %rbp
            popq %r8
            popq %r9
            popq %r10
            popq %r11
            popq %r12
            popq %r13
            popq %r14
            popq %r15
            movq 0(%rsp), %rsp
            # now we jump to the actual landing pad function
            ret
        ");
        core::hint::unreachable_unchecked();
    }
}

pub fn unwind(cause: Option<UnwindCause>) -> ! {
    println!("Starting unwind...");

    match start_unwinding(cause, 5) {
        Ok(_) => {
            unreachable!("");
        }
        Err(e) => {
            println!("Unwind failed: {:?}", e);
            loop {}
        }
    }
}

pub fn unwind_test() {
    println!("Unwind test is noop");
}

/////*
//// * Restore register and stack state right before the invocation
//// * make sure that all registers are restored (specifically, caller
//// * registers may be used for passing arguments). Hence we save the
//// * function pointer right below the stack (esp - 8) and jump to
//// * it from there.
//// *
//// * Note: interrupts are disabled in the kernel, NMIs are handled on a
//// * separate IST stack, so nothing should overwrite memory below the
//// * stack (i.e., esp - 8).
//// *
//// * %rdi -- pointer to Continuation
//// */
////
////global_asm!(
////    "  
////    .text 
////    .align  16              
////__unwind:
////    movq 16(%rdi), %rcx
////    movq 24(%rdi), %rdx
////    movq 32(%rdi), %rsi
////
////    movq 48(%rdi), %r8
////    movq 56(%rdi), %r9
////    movq 64(%rdi), %r10
////
////
////    movq 136(%rdi), %rsp
////    movq 128(%rdi), %rbp
////    movq 120(%rdi), %rbx
////    movq 112(%rdi), %r11
////    movq 104(%rdi), %r12
////    movq 96(%rdi), %r13
////    movq 88(%rdi), %r14
////    movq 80(%rdi), %r15
////    pushq 72(%rdi)
////    popfq
////
////    movq (%rdi), %rax
////    movq %rax, -8(%rsp)
////    movq 8(%rdi), %rax
////
////    movq 40(%rdi), %rdi
////
////    jmpq *-8(%rsp) "
////);
////
/////*
//// * Unwind test with simple functions
//// */
////#[no_mangle]
////pub fn foo(_x: u64, _y: u64) {
////    //unwind();
////    println!("you shouldn't see this");
////}
////
////#[no_mangle]
////pub fn foo_err(x: u64, y: u64) {
////    println!("foo was aborted, x:{}, y:{}", x, y);
////}
////
////extern "C" {
////    fn foo_tramp(x: u64, y: u64);
////}
////
//////trampoline!(foo);
////
/////*
//// * Unwind test with traits
//// */
////
////pub trait FooTrait {
////    fn simple_result(&self, x: u64) -> Result<u64, i64>;
////}
////
////pub struct Foo {
////    id: u64,
////}
////
////impl FooTrait for Foo {
////    fn simple_result(&self, _x: u64) -> Result<u64, i64> {
////        let r = self.id;
////        unwind();
////        Ok(r)
////    }
////}
////
////static FOO: Foo = Foo { id: 55 };
////
////#[no_mangle]
////pub extern "C" fn simple_result(s: &Foo, x: u64) -> Result<u64, i64> {
////    println!("simple_result: s.id:{}, x:{}", s.id, x);
////    let r = s.simple_result(x);
////    println!("simple_result: you shouldn't see this");
////    r
////}
////
////#[no_mangle]
////pub extern "C" fn simple_result_err(s: &Foo, x: u64) -> Result<u64, i64> {
////    println!("simple_result was aborted, s.id:{}, x:{}", s.id, x);
////    Err(-1)
////}
////
////extern "C" {
////    fn simple_result_tramp(s: &Foo, x: u64) -> Result<u64, i64>;
////}
////
//////trampoline!(simple_result);
////
////pub fn unwind_test() {
////    unsafe {
////        /*
////        foo_tramp(1, 2);
////        let r = simple_result_tramp(&FOO, 3);
////        match r {
////            Ok(n)  => println!("simple_result (ok):{}", n),
////            Err(e) => println!("simple_result, err: {}", e),
////        }
////        */
////    }
////}
