use super::Domain;
use crate::interrupt::{disable_irq, enable_irq};
use crate::syscalls::PDomain;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::mem::transmute;
use elfloader::ElfBinary;
use spin::Mutex;
use syscalls::Syscall;
//use syscalls::BootSyscall;
//use crate::syscalls::BOOT_SYSCALL;

fn sys_init_binary_range() -> (u64, u64) {
    extern "C" {
        static _binary_sys_init_build_init_start: u8;
        static _binary_sys_init_build_init_end: u8;
    }

    unsafe {
        (
            &_binary_sys_init_build_init_start as *const _ as u64,
            &_binary_sys_init_build_init_end as *const _ as u64,
        )
    }
}

pub unsafe fn load_sys_init() {
    let (binary_start, binary_end) = sys_init_binary_range();
    let binary_start: *const u8 = binary_start as *const u8;
    let binary_end: *const u8 = binary_end as *const u8;
    //type UserInit = fn(BootSyscall);
    type UserInit = fn(Box<dyn Syscall>);

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!(
        "start: {:x} end : {:x} ",
        binary_start as usize, binary_end as usize
    );

    // Create a new elf binary from the address range we just extracted
    let sys_init_elf = ElfBinary::new(
        "sys_init",
        core::slice::from_raw_parts(binary_start, num_bytes),
    )
    .expect("Got ELF file");

    // Create a domain for the to-be-loaded elf file
    let mut dom = Domain::new("sys_init");

    // load the binary
    sys_init_elf.load(&mut dom).expect("Cannot load binary");

    let offset = dom.offset().expect("Memory space for domain was not correctly allocated");

    // print its entry point for now
    println!(
        "entry point at {:x}",
        offset + sys_init_elf.entry_point()
    );

    let user_ep: UserInit = {
        let mut entry: *const u8 = offset.as_ptr();
        entry = entry.offset(sys_init_elf.entry_point() as isize);
        let _entry = entry as *const ();
        transmute::<*const (), UserInit>(_entry)
    };

    let pdom = Box::new(PDomain::new(dom));
    //BOOTING_DOMAIN.replace(Some(pdom));
    //user_ep(BOOT_SYSCALL);

    // Enable interrupts on exit to user so it can be preempted
    enable_irq();
    user_ep(pdom);
    disable_irq();

    println!("Hello back");
}
