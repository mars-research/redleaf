use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// Use the PIC 8259 crate 
// https://docs.rs/crate/pic8259_simple/0.1.1/source/src/lib.rs
use pic8259_simple::ChainedPics;
use spin;

use crate::{gdt, lapic, println};

// Map first PIC line to interrupt 32 
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

macro_rules! dummy_interrupt_handler {
    ($name: ident, $interrupt: expr) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut InterruptStackFrame) {
            println!("Interrupt {} triggered", $interrupt);
        }
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// See https://os.phil-opp.com/hardware-interrupts/ 
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });


lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);

		/* NMI fault hanler executes on the IST stack */
        unsafe {
            idt.non_maskable_interrupt
				.set_handler_fn(nmi_handler)
                .set_stack_index(gdt::NMI_IST_INDEX); 
        }

		/* Double fault hanler executes on the IST stack -- just in 
 		   case the kernel stack is already full and triggers a pagefault, 
		   that in turn (since the hardware will not be able to push the 
  	 	   exception fault on the stack will trigger a tripple fault */
        unsafe {
            idt.double_fault
				.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); 
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn init_irqs() {
    lapic::init();
    // unsafe { PICS.lock().initialize() };
}


extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("breakpoint:\n{:#?}", stack_frame);
    lapic::end_of_interrupt();
}

extern "x86-interrupt" fn nmi_handler(
    stack_frame: &mut InterruptStackFrame,
) {
    println!("nmi:\n{:#?}", stack_frame); 
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    println!("double fault:\n{:#?}", stack_frame);
	crate::halt(); 
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    print!(".");

    /*
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }*/
}

extern "x86-interrupt" fn apic_error_handler(stack_frame: &mut InterruptStackFrame) {
    println!("apic error:\n{:#?}", stack_frame);
    lapic::end_of_interrupt();

    /*
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }*/
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
