use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::lazy_static;
use x86::cpuid::CpuId;

use crate::{gdt, println, entryother};
use crate::drivers::Driver;
use crate::redsys::IRQRegistrar;
use crate::console::unlock_console; 

mod lapic;
mod ioapic;
mod pic;
pub mod idt; 
mod irq_manager;

pub use irq_manager::IRQManager;
use idt::{InterruptDescriptorTable, PtRegs, HandlerFunc};

pub const IRQ_OFFSET: u8 = 32;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = IRQ_OFFSET,
    Keyboard,
    ApicError = 19,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_u32(self) -> u32 {
        self as u32
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// Prototypes of ASM entry functions 
// (well, pt_regs as an argument is really a stretch)
extern {
    fn divide_error(pt_regs: &mut PtRegs);	  	 
    fn debug(pt_regs: &mut PtRegs);
    fn int3(pt_regs: &mut PtRegs);
    fn overflow(pt_regs: &mut PtRegs);		    
    fn bounds(pt_regs: &mut PtRegs);			
    fn invalid_op(pt_regs: &mut PtRegs);			
    fn device_not_available(pt_regs: &mut PtRegs);		
    fn double_fault(pt_regs: &mut PtRegs);			
    fn coprocessor_segment_overrun(pt_regs: &mut PtRegs);	
    fn invalid_TSS(pt_regs: &mut PtRegs);			
    fn segment_not_present(pt_regs: &mut PtRegs);		
    fn spurious_interrupt_bug(pt_regs: &mut PtRegs);	
    fn coprocessor_error(pt_regs: &mut PtRegs);		
    fn alignment_check(pt_regs: &mut PtRegs);		
    fn simd_coprocessor_error(pt_regs: &mut PtRegs);		

    fn stack_segment(pt_regs: &mut PtRegs);
    fn general_protection(pt_regs: &mut PtRegs);
    fn page_fault(pt_regs: &mut PtRegs);
    fn machine_check(pt_regs: &mut PtRegs);
    fn virtualization(pt_regs: &mut PtRegs);
    fn nmi_simple(pt_regs: &mut PtRegs);
}


lazy_static! {
    static ref irqManager: Arc<Mutex<IRQManager>> = {
        let arc = Arc::new(Mutex::new(IRQManager::new()));

        {
            let mut guard = arc.lock();
            guard.set_manager_handle(arc.clone());
        }

        arc
    };
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.divide_by_zero.set_handler_fn(divide_error);
        idt.debug.set_handler_fn(debug);
        //idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.breakpoint.set_handler_fn(int3);

        idt.overflow.set_handler_fn(overflow);
        idt.bound_range_exceeded.set_handler_fn(bounds);
        idt.invalid_opcode.set_handler_fn(invalid_op); 
        idt.device_not_available.set_handler_fn(device_not_available);

        idt.invalid_tss.set_handler_fn(invalid_TSS); 
        idt.segment_not_present.set_handler_fn(segment_not_present);
        idt.stack_segment_fault.set_handler_fn(stack_segment);

        idt.general_protection_fault.set_handler_fn(general_protection);
        
        #[cfg(not(feature="page_fault_on_ist"))]
        idt.page_fault.set_handler_fn(page_fault);
        
        idt.spurious_interrupt_bug.set_handler_fn(spurious_interrupt_bug);

        idt.x87_floating_point.set_handler_fn(coprocessor_error);
        idt.alignment_check.set_handler_fn(alignment_check);
        idt.machine_check.set_handler_fn(machine_check);

        idt.simd_floating_point.set_handler_fn(simd_coprocessor_error);
        idt.virtualization.set_handler_fn(virtualization);
        //idt.security_exception.set_handler_fn(security_exception_handler);

        /* Page fault hanler executes on the IST stack */
        #[cfg(feature="page_fault_on_ist")]
        unsafe {
            idt.page_fault
               .set_handler_fn(page_fault)
               .set_stack_index(gdt::PAGE_FAULT_IST_INDEX); 
        }


        /* NMI fault hanler executes on the IST stack */
        unsafe {
            idt.non_maskable_interrupt
               .set_handler_fn(nmi_simple)
               .set_stack_index(gdt::NMI_IST_INDEX); 
        }

        /* Double fault hanler executes on the IST stack -- just in 
            case the kernel stack is already full and triggers a pagefault, 
           that in turn (since the hardware will not be able to push the 
              exception fault on the stack will trigger a tripple fault */
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); 
        }

        extern {
            // The starting byte of the IRQ vectors
            static mut irq_entries_start: u64;
        }

        unsafe {

            let irq_handlers = & irq_entries_start as *const _ as u64; 
            println!("irq_entries_start:{:#x?}, irq_handlers:{:#x?}", irq_entries_start, irq_handlers); 
            for i in IRQ_OFFSET..255 {
                let ptr = (irq_handlers + 8*(i - IRQ_OFFSET) as u64) as *const ();
                let handler: HandlerFunc = core::mem::transmute(ptr);
                //idt[InterruptIndex::Timer.as_usize()].set_handler_fn(handler);
                idt[i as usize].set_handler_fn(handler);
            }
        }

        idt
    };
}

pub unsafe fn init_cpu(cpu: u32, stack: u64, code: u64) {
    let destination: *mut u8 = 0x7000 as *mut u8;

    let mut pgdir: u64;
    llvm_asm!("mov $0, cr3" : "=r"(pgdir) ::: "intel");

    println!("Cr3 {:x}", pgdir);
    entryother::copy_binary_to(destination);
    entryother::init_args(destination, stack, pgdir, code);

    println!("Starting CPU wth eip:{:x}, stack:{:x}", code, stack); 

    lapic::start_ap(cpu, destination);
}

pub fn init_idt() {
    IDT.load();

    //IDT.dump(); 

    // Trigger breakpoint interrupt to see that IDT is ok
    //x86_64::instructions::interrupts::int3();
}

pub fn init_irqs_local() {
    {
        if !detect_apic() {
            panic!("APIC is required to run RedLeaf");
        }
        pic::disable();
        lapic::init();
    }
}

pub fn init_irqs() {
    unsafe {
        ioapic::init();
        ioapic::irqen(1, 0);
    }
}

pub fn get_irq_manager() -> Arc<Mutex<IRQManager>> {
    irqManager.clone()
}

pub unsafe fn get_irq_registrar<T: Driver + Send>(driver: Arc<Mutex<T>>) -> IRQRegistrar<T> {
    IRQRegistrar::new(driver, irqManager.clone())
}

fn end_of_interrupt(#[allow(unused_variables)]interrupt: u8) {
    lapic::end_of_interrupt();
}

fn detect_apic() -> bool {
    let cpuid = CpuId::new();

    match cpuid.get_feature_info() {
        Some(feat) => feat.has_apic(),
        None => false,
    }
}

#[no_mangle]
extern fn do_divide_error(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Debug exception:\n{:#?}", pt_regs); 
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 1: Debug
#[no_mangle]
extern fn do_debug(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Debug exception:\n{:#?}", pt_regs); 
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 2: NMI
#[no_mangle]
extern fn do_nmi(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("NMI exception:\n{:#?}", pt_regs); 
}

// 3: Breakpoint
#[no_mangle]
extern fn do_int3(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Breakpoint exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
}

// 4: Overflow
#[no_mangle]
extern fn do_overflow(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Overflow exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 5: Bound range 
#[no_mangle]
extern fn do_bounds(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Bound range exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 6: Invalid opcode
#[no_mangle]
extern fn do_invalid_op(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Invalid opcode exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}


// 7: Device not available
#[no_mangle]
extern fn do_device_not_available(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Device not available exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 8: Double fault
#[no_mangle]
extern fn do_double_fault(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    println!("double fault:\n{:#?}", pt_regs);
    println!("Error Code {:x}", error_code);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 9: Old coprocessor error
#[no_mangle]
extern fn do_coprocessor_segment_overrun(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("old coprocessor segment overrun fault:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 10: Invalid TSS
#[no_mangle]
extern fn do_invalid_TSS(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    println!("Invalid TSS exception:\n{:#?}", pt_regs);
    println!("Error Code {:x}", error_code);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 11: Segment not present
#[no_mangle]
extern fn do_segment_not_present(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    println!("segment not present:\n{:#?}", pt_regs);
    println!("Error Code {:x}", error_code);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 12: #SS
#[no_mangle]
extern fn do_stack_segment(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    println!("stack segment fault:\n{:#?}", pt_regs);
    println!("Error Code {:x}", error_code);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 13: General protection
#[no_mangle]
extern fn do_general_protection(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    println!("general protection fault:\n{:#?}", pt_regs);
    println!("Error Code {:x}", error_code);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 14: Page fault 
#[no_mangle]
extern fn do_page_fault(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    use x86::controlregs::cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", unsafe { cr2() });
    println!("Error Code: {:x}", error_code);
    println!("{:#?}", pt_regs);

    crate::panic::backtrace_exception(pt_regs);
    crate::halt();
}

// 15: Spurious interrupt bug
#[no_mangle]
extern fn do_spurious_interrupt_bug(pt_regs: &mut PtRegs, error_code: isize) {
    println!("SPURIOUS INTERRUPT BUG");
    println!("Error Code: {:x}", error_code);
    println!("{:#?}", pt_regs);
}

// 16: x87 Floating-Point Exception
#[no_mangle]
extern fn do_coprocessor_error(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("x87 floating point exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 17: Alignment check
#[no_mangle]
extern fn do_alignment_check(pt_regs: &mut PtRegs, error_code: isize) {
    unlock_console(); 
    println!("Alignment check exception:\n{:#?}", pt_regs);
    println!("Error Code: {:x}", error_code);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 18: Machine check
// Note, in entry_64.S Linux redefines the function to machine_check_vector(%rip)
// We need to check what this means
#[no_mangle]
extern fn do_machine_check(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Machine check exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 19: SIMD Floating-Point Exception
#[no_mangle]
extern fn do_simd_coprocessor_error(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("SIMD Floating-Point Exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

// 20: Virtualization
#[no_mangle]
extern fn do_virtualization(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Virtualization exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}


// 30: Security 
#[no_mangle]
extern fn do_security(pt_regs: &mut PtRegs, _error_code: isize) {
    unlock_console(); 
    println!("Security exception:\n{:#?}", pt_regs);
    crate::panic::backtrace_exception(pt_regs);
    crate::halt(); 
}

static mut timer_count: u8 = 0;

use crate::panic::backtrace;
fn dump_proc(pt_regs: &PtRegs) {
    unsafe {
        timer_count += 1;

        if timer_count == 20 {
            timer_count  = 0;
            let rip = pt_regs.rip as *const u64 as *const u8;
            println!("rip 0x{:x} rsp 0x{:x}", pt_regs.rip, pt_regs.rsp);
            for x in 0..32 {
                print!("{:02x} ", *rip.offset(x) as u8);
            }
            println!("");
        }
    }
}

#[no_mangle]
extern fn do_IRQ(pt_regs: &mut PtRegs) -> u64 {
    let vector = pt_regs.orig_ax;

    // Jump to the handler here
    if vector == (InterruptIndex::Timer as u64) {
        // Timer (IRQ 0)
        timer_interrupt_handler(pt_regs);
        //dump_proc(&pt_regs);
    } else if vector >= (IRQ_OFFSET as u64) && vector <= 255 {
        // IRQs
        let irq: u8 = (vector - (IRQ_OFFSET as u64)) as u8;
        irqManager.lock().handle_irq(irq);
        end_of_interrupt(vector as u8);
    } else {
        // ???
        println!("Unknown interrupt: {}", vector); 
    }
    1 
}

// IRQ 0: Timer
fn timer_interrupt_handler(#[allow(unused_variables)]pt_regs: &mut PtRegs) {
    end_of_interrupt(InterruptIndex::Timer.as_u8());

    crate::waitqueue::signal_interrupt_threads(32); 
    crate::thread::schedule();
}

#[no_mangle]
extern fn enter_from_user_mode() {
    panic!("enter from user mode not supported");
    //crate::halt(); 
}

#[no_mangle]
extern fn prepare_exit_to_usermode() {
    panic!("prepare exit to user mode not supported");
    //crate::halt(); 
}

#[no_mangle]
extern fn panic_irq() {
    panic!("we don't support error_kernelspace in entry_64.S");
    //crate::halt(); 
}

#[no_mangle]
extern fn swapfs() {
    panic!("swapfs unsupported");
    //crate::halt(); 
}

#[no_mangle]
extern fn fixup_bad_iret(#[allow(unused_variables)]pt_regs: &mut PtRegs) -> u64 {
    panic!("fixup_bad_iret");
}

#[no_mangle]
extern fn sync_regs(pt_regs: &mut PtRegs) -> u64 {
    panic!("sync_regs:\n{:#?}", pt_regs);
    // Jump to the handler here
    //return 0
}

#[inline(always)]
pub fn disable_irq() {
    unsafe {
        x86::irq::disable();
    }
}

#[inline(always)]
pub fn enable_irq() {
    unsafe {
        x86::irq::enable();
    }
}

