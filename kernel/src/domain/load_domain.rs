use spin::Mutex;
use alloc::sync::Arc;
use elfloader::ElfBinary;
use super::trusted_binary;
use super::trusted_binary::SignatureCheckResult;
use super::domain::Domain;

pub unsafe fn load_domain(name: &str, binary_range: (*const u8, *const u8)) -> (Arc<Mutex<Domain>>, *const()) {
    let (binary_start, binary_end) = binary_range;

    let num_bytes = ((binary_end as usize) - (binary_start as usize)) as usize;

    println!("domain/{}: Binary start: {:x}, end: {:x} ",
        name, binary_start as usize, binary_end as usize);

    // Create a new elf binary from the address range we just extracted
    let binary = core::slice::from_raw_parts(binary_start, num_bytes);
    let domain_elf = ElfBinary::new(name, binary).expect("Invalid ELF file");

    // Verify signature in binary
    // FIXME: Actually enforce this
    match trusted_binary::verify(binary) {
        SignatureCheckResult::Unsigned => {
            println!("domain/{}: Binary is unsigned", name);
        },
        SignatureCheckResult::GoodSignature => {
            println!("domain/{}: Binary has good signature", name);
        },
        SignatureCheckResult::BadSignature => {
            println!("domain/{}: Binary has BAD signature", name);
        }
    }

    // Create a domain for the to-be-loaded elf file
    let dom = Arc::new(Mutex::new(Domain::new(name)));

    let mut loader = dom.lock();

    // load the binary
    domain_elf.load(&mut *loader).expect("Cannot load binary");

    // print its entry point for now
    println!("domain/{}: Entry point at {:x}",
        name, loader.offset + domain_elf.entry_point());

    println!("domain/{}: .text starts at {:x}",
        name, loader.offset + domain_elf.file.find_section_by_name(".text").unwrap().address());

    let user_ep: *const() = {
        let mut entry: *const u8 = (*loader).offset.as_ptr();
        entry = entry.offset(domain_elf.entry_point() as isize);
        let _entry = entry as *const ();
        _entry
    };

    // Drop the lock so if domain starts creating threads we don't
    // deadlock
    drop(loader);

    (dom, user_ep)
}
