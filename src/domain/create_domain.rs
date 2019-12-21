use elfloader::ElfBinary;
use super::Domain;
use alloc::string::String;
use crate::syscalls::UKERN;
use syscalls::Syscall;
use core::mem::transmute;
use crate::interrupt::{disable_irq, enable_irq};

pub unsafe fn create_domain(name: &'static str, binary_range: (*const u8, *const u8)) {
    let (binary_start, binary_end) = binary_range;
    type user_init = fn(Syscall);

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!("domain/{}: Binary start: {:x}, end: {:x} ", name, binary_start as usize, binary_end as usize);

    // Create a new elf binary from the address range we just extracted
    let domain_elf = ElfBinary::new(name, core::slice::from_raw_parts(binary_start, num_bytes)).expect("Invalid ELF file");

    // Create a domain for the to-be-loaded elf file
    let mut loader = Domain::new(String::from(name));

    // load the binary
    domain_elf.load(&mut loader).expect("Cannot load binary");

    // print its entry point for now
    println!("domain/{}: Entry point at {:x}", name, loader.offset + domain_elf.entry_point());

    let user_ep: user_init = unsafe {
        let mut entry: *const u8 = loader.offset.as_ptr();
        entry = entry.offset(domain_elf.entry_point() as isize);
        let _entry = entry as *const ();
        transmute::<*const(), user_init>(_entry)
    };

    // Enable interrupts as we do upcall into user
    enable_irq();
    user_ep(UKERN);
    disable_irq(); 

    println!("domain/{}: Returned", name);
}
