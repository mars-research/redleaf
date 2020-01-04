//mod interrupt;

use backtracer;
use crate::interrupt::idt::PtRegs;
use core::panic::PanicInfo;
use spin::Once;

use alloc::rc::Rc;
use addr2line;
use addr2line::gimli;
use addr2line::Context;


#[inline(always)]
pub fn backtrace_exception_no_resolve(pt_regs:&mut PtRegs) {
    println!("Backtrace:");

    backtracer::trace_from(backtracer::EntryPoint::new(pt_regs.rbp, pt_regs.rsp, pt_regs.rip), |frame| {
        let ip = frame.ip();
        println!("ip:{:?}", ip);
        true        // xxx
    });
}

#[inline(always)]
pub fn backtrace_exception(pt_regs:&mut PtRegs) {
    println!("Backtrace:");

    let elf_data = match ELF_DATA.r#try() {
        Some(t) => t,
        None => {
            println!("ELF_DATA was not initialized");  
            backtrace_exception_no_resolve(pt_regs); 
            return;
        }
    };
        
    let relocated_offset = RELOCATED_OFFSET;
    let elf_binary =
        elfloader::ElfBinary::new("kernel", &elf_data).expect("Can't parse kernel binary.");
    let context = new_ctxt(&elf_binary);


    let mut count = 0;
    backtracer::trace_from(backtracer::EntryPoint::new(pt_regs.rbp, pt_regs.rsp, pt_regs.rip), |frame| {
        count += 1;
        backtrace_format(context.as_ref(), relocated_offset, count, frame)
    });

}

pub fn backtrace_no_resolve() {

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

static ELF_DATA:Once<&'static [u8]> = Once::new();  
static RELOCATED_OFFSET: u64 = 0x100_000;

pub fn init_backtrace(elf_data: &'static [u8]) {
    ELF_DATA.call_once(|| elf_data);
}

fn new_ctxt(file: &elfloader::ElfBinary) -> Option<Context> {
    let endian = gimli::RunTimeEndian::Little;

    fn load_section<S, Endian>(elf: &elfloader::ElfBinary, endian: Endian) -> S
    where
        S: gimli::Section<gimli::EndianRcSlice<Endian>>,
        Endian: gimli::Endianity,
    {
        let data = elf
            .file
            .find_section_by_name(S::section_name())
            .map(|s| s.raw_data(&elf.file))
            .unwrap_or(&[]);
        S::from(gimli::EndianRcSlice::new(Rc::from(&*data), endian))
    }

    let debug_abbrev: gimli::DebugAbbrev<_> = load_section(file, endian);
    let debug_addr: gimli::DebugAddr<_> = load_section(file, endian);
    let debug_info: gimli::DebugInfo<_> = load_section(file, endian);
    let debug_line: gimli::DebugLine<_> = load_section(file, endian);
    let debug_line_str: gimli::DebugLineStr<_> = load_section(file, endian);
    let debug_ranges: gimli::DebugRanges<_> = load_section(file, endian);
    let debug_rnglists: gimli::DebugRngLists<_> = load_section(file, endian);
    let debug_str: gimli::DebugStr<_> = load_section(file, endian);
    let debug_str_offsets: gimli::DebugStrOffsets<_> = load_section(file, endian);
    let default_section = gimli::EndianRcSlice::new(Rc::from(&[][..]), endian);

    Context::from_sections(
        debug_abbrev,
        debug_addr,
        debug_info,
        debug_line,
        debug_line_str,
        debug_ranges,
        debug_rnglists,
        debug_str,
        debug_str_offsets,
        default_section,
    )
    .ok()
}

fn backtrace_format(
    context: Option<&Context>,
    relocated_offset: u64,
    count: usize,
    frame: &backtracer::Frame,
) -> bool {
    let ip = frame.ip();
    println!("frame #{:<2} - {:#02$x}", count, ip as usize, 20);
    let mut resolved = false;

    backtracer::resolve(context, relocated_offset, ip, |symbol| {
        if !resolved {
            resolved = true;
        } else {
            print!("                                ");
        }
        if let Some(name) = symbol.name() {
            if name.as_bytes().len() == 0 {
                print!(" - <empty>");
            } else {
                print!(" - {}", name);
                if let Some(file) = symbol.filename() {
                    print!(" ({}", file);
                    if let Some(line) = symbol.lineno() {
                        print!(":{})", line);
                    } else {
                        print!(")");
                    }
                }
            }
        } else {
            println!(" - <unknown>");
        }
        println!("");
    });

    if !resolved {
        println!(" - <no info>");
    }
    true
}


#[inline(always)]
pub fn backtrace() {

    let elf_data = match ELF_DATA.r#try() {
        Some(t) => t,
        None => {
            println!("ELF_DATA was not initialized");  
            backtrace_no_resolve(); 
            return;
        }
    };
 

    println!("Backtrace:");

    let elf_data = ELF_DATA.r#try().expect("ELF_DATA was not initialized");  
    let relocated_offset = RELOCATED_OFFSET;
    let elf_binary =
        elfloader::ElfBinary::new("kernel", &elf_data).expect("Can't parse kernel binary.");
    let context = new_ctxt(&elf_binary);

    let mut count = 0;
    backtracer::trace(|frame| {
        count += 1;
        backtrace_format(context.as_ref(), relocated_offset, count, frame)
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


