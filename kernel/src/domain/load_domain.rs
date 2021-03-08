use super::Domain;
use super::trusted_binary;
use super::trusted_binary::SignatureCheckResult;
use alloc::sync::Arc;
use elfloader::ElfBinary;
use spin::Mutex;

pub unsafe fn load_domain(
    name: &str,
    binary_range: (*const u8, *const u8),
) -> (Domain, *const ()) {
    let (binary_start, binary_end) = binary_range;

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!(
        "domain/{}: Binary start: {:x}, end: {:x} ",
        name, binary_start as usize, binary_end as usize
    );

    // Create a new elf binary from the address range we just extracted
    let binary_vec: alloc::vec::Vec<u8>;

    #[cfg(not(debug_assertions))]
    let binary = core::slice::from_raw_parts(binary_start, num_bytes);
    // Align the binary at page boundary when building in debug mode
    #[cfg(debug_assertions)]
    let binary = {
        binary_vec = unsafe {
            use alloc::vec::Vec;
            use core::alloc::Layout;

            let layout = Layout::from_size_align(num_bytes, 4096)
                .map_err(|e| panic!("Layout error: {}", e))
                .unwrap();

            let elf_buf = unsafe { alloc::alloc::alloc(layout) as *mut u8 };
            let mut v: Vec<u8> = unsafe { Vec::from_raw_parts(elf_buf, num_bytes, num_bytes) };
            core::ptr::copy(binary_start, v.as_mut_ptr(), num_bytes);
            v
        };
        binary_vec.as_slice()
    };

    let domain_elf = ElfBinary::new(name, binary).expect("Invalid ELF file");

    // Verify signature in binary
    // FIXME: Actually enforce this
    match trusted_binary::verify(binary) {
        SignatureCheckResult::Unsigned => {
            println!("domain/{}: Binary is unsigned", name);
        }
        SignatureCheckResult::GoodSignature => {
            println!("domain/{}: Binary has good signature", name);
        }
        SignatureCheckResult::BadSignature => {
            println!("domain/{}: Binary has BAD signature", name);
        }
    }

    // Create a domain for the to-be-loaded elf file
    let mut dom = Domain::new(name);

    // load the binary
    let entry_point = domain_elf.entry_point();
    let text_address = domain_elf.file
        .find_section_by_name(".text")
        .unwrap()
        .address();
    dom.load_elf(domain_elf).expect("Cannot load binary");

    let offset = dom.offset().expect("Memory space for domain was not correctly allocated");

    // print its entry point for now
    println!(
        "domain/{}: Entry point at {:x}",
        name,
        offset + entry_point,
    );

    println!(
        "domain/{}: .text starts at {:x}",
        name,
        offset + text_address,
    );

    let user_ep: *const () = {
        let mut entry: *const u8 = offset.as_ptr();
        entry = entry.offset(entry_point as isize);
        let _entry = entry as *const ();
        _entry
    };

    (dom, user_ep)
}
