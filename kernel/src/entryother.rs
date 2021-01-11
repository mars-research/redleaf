// Rust wrapper to access entryother.asm

extern "C" {
    // not actually meant to be called
    fn _binary_build_entryother_bin_start();
    fn _binary_build_entryother_bin_end();
    fn start_others();
}

pub const BINARY_START: *const u8 = _binary_build_entryother_bin_start as *const u8;
pub const BINARY_END: *const u8 = _binary_build_entryother_bin_end as *const u8;
pub const START_OTHERS: *const u8 = start_others as *const u8;

pub unsafe fn copy_binary_to(destination: *mut u8) {
    let count: isize = ((BINARY_END as u32) - (BINARY_START as u32)) as isize;
    for offset in 0isize..count {
        *destination.offset(offset) = *BINARY_START.offset(offset);
    }
}

pub unsafe fn init_args(destination: *mut u8, stack: u64, pgdir: u64, code: u64) {
    let stackp: *mut u64 = destination.offset(-8) as *mut u64;
    let pgdirp: *mut u64 = destination.offset(-16) as *mut u64;
    let codep: *mut u64 = destination.offset(-24) as *mut u64;
    let bootasm_start: *mut u64 = destination.offset(-32) as *mut u64;

    println!(
        "Bootasm start_others is at {:x}",
        START_OTHERS as *const u64 as u64
    );
    *stackp = stack;
    *pgdirp = pgdir;
    *codep = code;
    *bootasm_start = START_OTHERS as *const u64 as u64;
}
