//mod interrupt;

use backtracer;
use crate::interrupt::idt::PtRegs;
use core::panic::PanicInfo;


#[inline(always)]
pub fn backtrace_exception(pt_regs:&mut PtRegs) {
    println!("Backtrace:");

    backtracer::trace_from(backtracer::EntryPoint::new(pt_regs.rbp, pt_regs.rsp, pt_regs.rip), |frame| {
        let ip = frame.ip();
        println!("ip:{:?}", ip);
        true        // xxx
    });
}

pub fn backtrace() {

    backtracer::trace(|frame| {
        let ip = frame.ip();
        /*
        let symbol_address = frame.symbol_address();

        extern {
            /// The starting byte of the thread data segment
            static mut __text_start: u8;
        }



        // Resolve this instruction pointer to a symbol name
        backtracer::resolve(__text_start, ip, |symbol| {
            if let Some(name) = symbol.name() {
                println!("{}", name); 
            }
            if let Some(filename) = symbol.filename() {
                println!("{}", filename); 
            }
        });*/
        println!("ip:{:?}", ip); 

        true // keep going to the next frame
    });
}

#[cfg_attr(target_os = "none", panic_handler)]
#[no_mangle]
pub fn panic_impl(info: &PanicInfo) -> ! {
    println!("Panic:");
    if let Some(message) = info.message() {
        println!(": '{}'", message);
    }
    if let Some(location) = info.location() {
        println!(" in {}:{}", location.file(), location.line());
    } else {
        println!();
    }

    backtrace();

    crate::halt(); 
}


