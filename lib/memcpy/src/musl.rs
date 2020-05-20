extern {
    pub fn memcpy(dest: *mut u8, src: *const u8,
                            n: usize) -> *mut u8;
    fn memmove(dest: *mut u8, src: *const u8,
                             n: usize) -> *mut u8;
    fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8;
    fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32;
}
