use gimli::{BaseAddresses, EhFrame, EndianSlice, NativeEndian, UnwindSection, UninitializedUnwindContext};
use core::slice;
use console::println;

extern "C" {
    static __text_start: usize;
    static __eh_frame_hdr_start: usize;
    static __eh_frame_hdr_end: usize;
    static __eh_frame_start: usize;
    static __eh_frame_end: usize;
    static __got_start: usize;
    static __got_end: usize;
}

unsafe fn get_eh_frame() -> EhFrame<EndianSlice<'static, NativeEndian>> {
    let eh_frame_start = &__eh_frame_start as *const _ as *const u8;
    let eh_frame_end = &__eh_frame_end as *const _ as *const u8;

    let eh_frame_len = eh_frame_end.offset_from(eh_frame_start) as usize;
    let eh_frame_slice: &'static [u8] = slice::from_raw_parts(eh_frame_start, eh_frame_len);

    EhFrame::new(eh_frame_slice, NativeEndian)
}

unsafe fn get_base_addresses() -> BaseAddresses {
    let text_start = &__text_start as *const _ as u64;
    let got_start = &__got_start as *const _ as u64;
    let eh_frame_start = &__eh_frame_start as *const _ as u64;
    let eh_frame_hdr_start = &__eh_frame_hdr_start as *const _ as u64;

    let bases = BaseAddresses::default()
        .set_text(text_start)
        .set_got(got_start)
        .set_eh_frame(eh_frame_start)
        .set_eh_frame_hdr(eh_frame_hdr_start)
    ;

    println!("UNWIND: Got .text offset {:x?}", text_start);
    println!("UNWIND: Got .got offset {:x?}", got_start);

    bases
}

pub fn test() {
    let eh_frame = unsafe { get_eh_frame() };
    let bases = unsafe { get_base_addresses() };

    panic!("TEST PANIC");
    println!("Start of list");

    let mut entries = eh_frame.entries(&bases);
    while let Some(entry) = entries.next().unwrap() {
        use gimli::CieOrFde::*;
        match entry {
            Cie(cie) => {
                println!("Got CIE: {:?}", cie);
            }
            Fde(fde) => {
                println!("Got FDE: {:?}", fde);
                let parsed = fde.parse(EhFrame::cie_from_offset);
                println!("Fully parsed: {:#x?}", parsed);
            }
        }
    }

    println!("End of list");
}
