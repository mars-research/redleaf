use alloc::string::String;
use alloc::string::ToString;
//use core::cell::RefCell;
use super::super::memory::paddr_to_kernel_vaddr;
use crate::alloc::vec::Vec;
use crate::arch::vspace::{MapAction, ResourceType, VSpace};
use crate::memory::VSPACE;
use crate::thread::Thread;
use alloc::sync::Arc;
use log::{debug, info, trace};
use spin::Mutex;
use x86::bits64::paging::{PAddr, VAddr, BASE_PAGE_SHIFT, BASE_PAGE_SIZE};
//use alloc::rc::Rc;
use crate::heap::PHeap;
use crate::syscalls::PDomain;
use crate::{is_page_aligned, round_up};
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use libsyscalls;
use spin::Once;

/// This should be a cryptographically secure number, for now
/// just sequential ID
static DOMAIN_ID: AtomicU64 = AtomicU64::new(0);

/// Global Domain list
pub static KERNEL_DOMAIN: Once<Arc<Mutex<Domain>>> = Once::new();

#[cfg(feature = "gdb_domain_variables")]
#[no_mangle]
#[inline]
/// This is a dummy function. It exists only to have a breakpoint set on it which will allow the gdb helper script to handle changes
pub(crate) fn gdb_notify_new_domain_loaded() {
    // println!("GDB NOTIFY");
}

//#[thread_local]
//pub static BOOTING_DOMAIN: RefCell<Option<Box<PDomain>>> = RefCell::new(None);

pub struct Domain {
    pub id: u64,
    pub name: String,
    pub mapping: Vec<(VAddr, usize, u64, MapAction)>,
    /// Offset where ELF is located.
    pub offset: VAddr,
    /// The entry point of the ELF file.
    pub entry_point: VAddr,
    /// List of threads in the domain
    //threads: Option<Arc<Mutex<Rc<RefCell<Thread>>>>>,
    threads: DomainThreads,
}

pub struct DomainThreads {
    head: Option<Arc<Mutex<Thread>>>,
}

unsafe impl Send for DomainThreads {}
//unsafe impl Sync for DomainThreads {}

impl DomainThreads {
    pub fn new() -> DomainThreads {
        DomainThreads { head: None }
    }
}

impl Domain {
    pub fn new(name: &str) -> Domain {
        Domain {
            id: DOMAIN_ID.fetch_add(1, Ordering::SeqCst),
            name: name.to_string(),
            mapping: Vec::with_capacity(64),
            offset: VAddr::from(0usize),
            entry_point: VAddr::from(0usize),
            threads: DomainThreads::new(),
        }
    }

    /// This function should be executed under a lock on domain
    /// We explicitly avoid using another lock, but the assumption
    /// is that it's imposible to access the domain without holding
    /// a lock on the domain data structure
    pub fn add_thread(&mut self, t: Arc<Mutex<Thread>>) {
        let previous_head = self.threads.head.take();

        if let Some(node) = previous_head {
            t.lock().next_domain = Some(node);
        }

        self.threads.head = Some(t);
    }
}

/// Create kernel domain (must be called before any threads are
/// created)
pub fn init_domains() {
    let kernel = Arc::new(Mutex::new(Domain::new("kernel")));
    libsyscalls::syscalls::init(Box::new(PDomain::new(Arc::clone(&kernel))));
    KERNEL_DOMAIN.call_once(|| kernel);

    // init global references to syscalls (mostly for RRef deallocation)
    interface::rref::init(Box::new(PHeap::new()), 0);
}

impl elfloader::ElfLoader for Domain {
    /// Makes sure the domain vspace is backed for the regions
    /// reported by the ELF loader as loadable.
    ///
    /// Our strategy is to first figure out how much space we need,
    /// then allocate a single chunk of physical memory and
    /// map the individual pieces of it with different access rights.
    /// This has the advantage that our address space is
    /// all a very simple 1:1 mapping of physical memory.
    fn allocate(&mut self, load_headers: elfloader::LoadableHeaders) -> Result<(), &'static str> {
        // Should contain what memory range we need to cover to contain
        // loadable regions:
        let mut min_base: VAddr = VAddr::from(usize::max_value());
        let mut max_end: VAddr = VAddr::from(0usize);
        let mut max_alignment: u64 = 0x1000;

        for header in load_headers.into_iter() {
            let base = header.virtual_addr();
            let size = header.mem_size() as usize;
            let align_to = header.align();
            let flags = header.flags();

            // Calculate the offset and align to page boundaries
            // We can't expect to get something that is page-aligned from ELF
            let page_base: VAddr = VAddr::from(base & !0xfff); // Round down to nearest page-size
            let size_page = round_up!(size + (base & 0xfff) as usize, BASE_PAGE_SIZE as usize);
            assert!(size_page >= size);
            assert_eq!(size_page % BASE_PAGE_SIZE, 0);
            assert_eq!(page_base % BASE_PAGE_SIZE, 0);

            // Update virtual range for ELF file [max, min] and alignment:
            if max_alignment < align_to {
                max_alignment = align_to;
            }
            if min_base > page_base {
                min_base = page_base;
            }
            if page_base + size_page as u64 > max_end {
                max_end = page_base + size_page as u64;
            }

            debug!(
                "ELF Allocate: {:#x} -- {:#x} align to {:#x}",
                page_base,
                page_base + size_page,
                align_to
            );

            let map_action = match (flags.is_execute(), flags.is_write(), flags.is_read()) {
                (false, false, false) => panic!("MapAction::None"),
                (true, false, false) => panic!("MapAction::None"),
                (false, true, false) => panic!("MapAction::None"),
                (false, false, true) => MapAction::ReadUser,
                (true, false, true) => MapAction::ReadExecuteUser,
                (true, true, false) => panic!("MapAction::None"),
                (false, true, true) => MapAction::ReadWriteUser,
                (true, true, true) => MapAction::ReadWriteExecuteUser,
            };

            // We don't allocate yet -- just record the allocation parameters
            // This has the advantage that we know how much memory we need
            // and can reserve one consecutive chunk of physical memory
            self.mapping
                .push((page_base, size_page, align_to, map_action));
        }

        assert!(
            is_page_aligned!(min_base),
            "min base is not aligned to page-size"
        );
        assert!(
            is_page_aligned!(max_end),
            "max end is not aligned to page-size"
        );
        let pbase = VSpace::allocate_pages_aligned(
            ((max_end - min_base) >> BASE_PAGE_SHIFT) as usize,
            ResourceType::Binary,
            max_alignment,
        );

        let ptr = pbase.as_u64() as *mut u8;
        for i in 0..((max_end.as_usize() - min_base.as_usize()) as isize) {
            unsafe {
                *ptr.offset(i) = 0;
            }
        }
        println!("num_pages: {}", (max_end - min_base) >> BASE_PAGE_SHIFT);

        self.offset = VAddr::from(pbase.as_usize());
        info!(
            "Binary loaded at address: {:#x} entry {:#x}",
            self.offset, self.entry_point
        );

        // XXX: Pages are already mapped on the global vspace. We do not need to map it again. But
        // for security reasons, we need to change the permission bits of those pages and restore
        // it when we free those pages
        //for (_base, size, _alignment, action) in self.mapping.iter() {
        //self.vspace
        //    .map_generic(self.offset, (pbase, *size), *action)
        //    .expect("Can't map ELF region");
        //}

        Ok(())
    }

    /// Load a region of bytes into the virtual address space of the process.
    fn load(&mut self, destination: u64, region: &[u8]) -> Result<(), &'static str> {
        let destination = self.offset + destination;
        trace!(
            "ELF Load at {:#x} -- {:#x}",
            destination,
            destination + region.len()
        );

        // Load the region at destination in the kernel space
        for (idx, val) in region.iter().enumerate() {
            let vaddr = VAddr::from(destination + idx);
            let paddr = {
                let mut _paddr: PAddr = PAddr::from(0 as usize);
                {
                    let ref mut vspace = *VSPACE.lock();
                    _paddr = vspace.resolve_addr(vaddr).expect("Can't resolve address");
                };
                _paddr
            };

            // TODO: Inefficient byte-wise copy
            // If this is allocated as a single block of physical memory
            // we can just do paddr_to_vaddr and memcopy
            let ptr = paddr.as_u64() as *mut u8;
            unsafe {
                *ptr = *val;
            }
        }

        Ok(())
    }

    /// Relocating the symbols.
    ///
    /// Since the binary is a position independent executable that is 'statically' linked
    /// with all dependencies we only expect to get relocations of type RELATIVE.
    /// Otherwise, the build would be broken or you got a garbage ELF file.
    /// We return an error in this case.
    fn relocate(&mut self, entry: &elfloader::Rela<elfloader::P64>) -> Result<(), &'static str> {
        // Get the pointer to where the relocation happens in the
        // memory where we loaded the headers
        // The forumla for this is our offset where the kernel is starting,
        // plus the offset of the entry to jump to the code piece
        let addr = self.offset + entry.get_offset();

        // Translate `addr` into a kernel vaddr we can write to:
        let paddr = {
            let mut _paddr: PAddr = PAddr::from(0 as usize);
            {
                let ref mut vspace = *VSPACE.lock();
                _paddr = vspace.resolve_addr(addr).expect("Can't resolve address");
            }
            _paddr
        };
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);

        debug!("ELF relocation paddr {:#x} kernel_addr {:#x}", paddr, vaddr);

        use elfloader::TypeRela64;
        if let TypeRela64::R_RELATIVE = TypeRela64::from(entry.get_type()) {
            // This is a relative relocation of a 64 bit value, we add the offset (where we put our
            // binary in the vspace) to the addend and we're done:
            unsafe {
                // Scary unsafe changing stuff in random memory locations based on
                // ELF binary values weee!
                *(vaddr.as_mut_ptr::<u64>()) = self.offset.as_u64() + entry.get_addend();
            }
            Ok(())
        } else {
            Err("Can only handle R_RELATIVE for relocation")
        }
    }

    fn make_readonly(&mut self, base: u64, size: usize) -> Result<(), &'static str> {
        trace!(
            "Make readonly {:#x} -- {:#x}",
            self.offset + base,
            self.offset + base + size
        );
        assert_eq!(
            (self.offset + base + size) % BASE_PAGE_SIZE,
            0,
            "RELRO segment doesn't end on a page-boundary"
        );

        let _from: VAddr = self.offset + (base & !0xfff); // Round down to nearest page-size
        let _to = self.offset + base + size;
        Ok(())
    }
}
