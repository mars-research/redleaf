#[derive(Clone, Copy, Debug)]
#[repr(C, packed)] // only repr(C) would add unwanted padding before first_section
pub struct CommandLineTag {
    typ: u32,
    size: u32,
    string: u8,
}

impl CommandLineTag {
    pub fn cmdline(&self) -> &str {
        use core::{mem, slice, str};
        unsafe {
            let strlen = self.size as usize - mem::size_of::<CommandLineTag>();
            str::from_utf8_unchecked(slice::from_raw_parts((&self.string) as *const u8, strlen))
        }
    }
}
