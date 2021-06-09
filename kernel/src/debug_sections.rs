//! Kernel debug sections.

use core::concat_idents;

macro_rules! section {
    ($section:ident) => {
        unsafe {
            let section_start = &concat_idents!(__, $section, _start);
            let section_end = &concat_idents!(__, $section, _end);

            let section_size = (section_start as *const _ as usize) - (section_end as *const _ as usize);

            println!("I think {} is at 0x{:x?}", stringify!($section), section_start as *const _ as usize);

            core::slice::from_raw_parts(section_start, section_size)
        }
    };
}

extern "C" {
    static __debug_abbrev_start: u8;
    static __debug_abbrev_end: u8;
    static __debug_addr_start: u8;
    static __debug_addr_end: u8;
    static __debug_info_start: u8;
    static __debug_info_end: u8;
    static __debug_line_start: u8;
    static __debug_line_end: u8;
    static __debug_line_str_start: u8;
    static __debug_line_str_end: u8;
    static __debug_ranges_start: u8;
    static __debug_ranges_end: u8;
    static __debug_rnglists_start: u8;
    static __debug_rnglists_end: u8;
    static __debug_str_start: u8;
    static __debug_str_end: u8;
    static __debug_str_offsets_start: u8;
    static __debug_str_offsets_end: u8;
}

pub unsafe fn debug_abbrev() -> &'static [u8] {
    section!(debug_abbrev)
}

pub unsafe fn debug_addr() -> &'static [u8] {
    section!(debug_addr)
}

pub unsafe fn debug_info() -> &'static [u8] {
    section!(debug_info)
}

pub unsafe fn debug_line() -> &'static [u8] {
    section!(debug_line)
}

pub unsafe fn debug_line_str() -> &'static [u8] {
    section!(debug_line_str)
}

pub unsafe fn debug_ranges() -> &'static [u8] {
    section!(debug_ranges)
}

pub unsafe fn debug_rnglists() -> &'static [u8] {
    section!(debug_rnglists)
}

pub unsafe fn debug_str() -> &'static [u8] {
    section!(debug_str)
}

pub unsafe fn debug_str_offsets() -> &'static [u8] {
    section!(debug_str_offsets)
}
