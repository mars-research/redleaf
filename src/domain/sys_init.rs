use crate::memory::VSPACE;
use x86::bits64::paging::BASE_PAGE_SIZE;
use elfloader::ElfBinary;
use super::Domain;
use alloc::string::String;
use crate::syscalls::UKERN;
use syscalls::syscalls::Syscall;
use core::mem::transmute;

macro_rules! round_up {
    ($num:expr, $s:expr) => {
        (($num + $s - 1) / $s) * $s
    };
}

fn sys_init_binary_range() -> (u64, u64) {
    extern "C" {
        static _binary_sys_init_build_init_start: u8;
        static _binary_sys_init_build_init_end: u8;
    }

    unsafe {
        (
            & _binary_sys_init_build_init_start as *const _ as u64,
            & _binary_sys_init_build_init_end as *const _ as u64
        )
    }
}

pub unsafe fn load_sys_init() {
    let (binary_start, binary_end) = sys_init_binary_range();
    let binary_start: *const u8 = binary_start as *const u8;
    let binary_end: *const u8 = binary_end as *const u8;
    type user_init = fn(Syscall);

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!("start: {:x} end : {:x} ", binary_start as usize, binary_end as usize);

    // Create a new elf binary from the address range we just extracted
    let sys_init_elf = ElfBinary::new("sys_init", core::slice::from_raw_parts(binary_start, num_bytes)).expect("Got ELF file");

    // Create a domain for the to-be-loaded elf file
    let mut loader = Domain::new(String::from("sys_init"));

    // load the binary
    sys_init_elf.load(&mut loader).expect("Cannot load binary");

    // print its entry point for now
    println!("entry point at {:x}", loader.offset + sys_init_elf.entry_point());

    let user_ep: user_init = unsafe {
        let mut entry: *const u8 = loader.offset.as_ptr();
        entry = entry.offset(sys_init_elf.entry_point() as isize);
        let _entry = entry as *const ();
        transmute::<*const(), user_init>(_entry)
    };
    user_ep(UKERN);
}
