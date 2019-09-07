// Rust wrapper to access entryother.asm

extern {
    // not actually meant to be called
    fn _binary_build_entryother_bin_start();
    fn _binary_build_entryother_bin_end();
}

pub const BINARY_START: *const u8 = _binary_build_entryother_bin_start as *const u8;
pub const BINARY_END: *const u8 = _binary_build_entryother_bin_end as *const u8;

pub unsafe fn copy_binary_to(destination: *mut u8) {
  let count: isize = ((BINARY_END as u32) - (BINARY_START as u32)) as isize;
  for offset in 0isize..count {
    *destination.offset(offset) = *BINARY_START.offset(offset);
  }
}

pub unsafe fn init_args(destination: *mut u8, stack: u32, pgdir: u32, code: u64) {
  let stackp: *mut u32 = destination.offset(-4) as *mut u32;
  let pgdirp: *mut u32 = destination.offset(-8) as *mut u32;
  let codep: *mut u64 = destination.offset(-16) as *mut u64;

  *stackp = stack;
  *pgdirp = pgdir;
  *codep = code;
}