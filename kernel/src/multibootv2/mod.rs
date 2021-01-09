#![deny(missing_debug_implementations)]

use core::fmt;

pub use boot_loader_name::BootLoaderNameTag;
pub use command_line::CommandLineTag;
use header::{Tag, TagIter};
pub use memory_map::{MemoryArea, MemoryAreaIter, MemoryMapTag};
pub use module::{ModuleIter, ModuleTag};

use crate::round_up;

mod boot_loader_name;
mod command_line;
mod header;
mod memory_map;
mod module;

use crate::arch::memory::BASE_PAGE_SIZE;
use crate::arch::{kernel_end, KERNEL_END};

pub unsafe fn load(address: usize) -> BootInformation {
    assert_eq!(0, address & 0b111);
    let multiboot = &*(address as *const BootInformationInner);

    assert_eq!(0, multiboot.total_size & 0b111);
    assert!(multiboot.has_valid_end_tag());

    // put the multibootv2 header after kernel end
    core::ptr::copy(
        address as *const usize as *const u8,
        kernel_end() as *mut u8,
        multiboot.total_size as usize,
    );
    let new_end = kernel_end() + multiboot.total_size as u64;
    KERNEL_END = round_up!(new_end, BASE_PAGE_SIZE as u64);

    let kernel_end_ptr = kernel_end() as *const u64 as *const u8;
    let _multiboot = &*(kernel_end_ptr as *const BootInformationInner);

    BootInformation {
        inner: _multiboot,
        offset: 0,
    }
}

pub unsafe fn load_with_offset(address: usize, offset: usize) -> BootInformation {
    if !cfg!(test) {
        assert_eq!(0, address & 0b111);
        assert_eq!(0, offset & 0b111);
    }
    let multiboot = &*((address + offset) as *const BootInformationInner);
    assert_eq!(0, multiboot.total_size & 0b111);
    assert!(multiboot.has_valid_end_tag());
    BootInformation {
        inner: multiboot,
        offset,
    }
}

pub struct BootInformation {
    inner: *const BootInformationInner,
    offset: usize,
}

#[repr(C, packed)]
struct BootInformationInner {
    total_size: u32,
    _reserved: u32,
}

impl BootInformation {
    pub fn start_address(&self) -> usize {
        self.inner as usize
    }

    pub fn end_address(&self) -> usize {
        self.start_address() + self.total_size()
    }

    pub fn total_size(&self) -> usize {
        self.get().total_size as usize
    }

    pub fn memory_map_tag<'a>(&'a self) -> Option<&'a MemoryMapTag> {
        self.get_tag(6)
            .map(|tag| unsafe { &*(tag as *const Tag as *const MemoryMapTag) })
    }

    pub fn module_tags(&self) -> ModuleIter {
        module::module_iter(self.tags())
    }

    pub fn boot_loader_name_tag<'a>(&'a self) -> Option<&'a BootLoaderNameTag> {
        self.get_tag(2)
            .map(|tag| unsafe { &*(tag as *const Tag as *const BootLoaderNameTag) })
    }

    pub fn command_line_tag<'a>(&'a self) -> Option<&'a CommandLineTag> {
        self.get_tag(1)
            .map(|tag| unsafe { &*(tag as *const Tag as *const CommandLineTag) })
    }

    fn get(&self) -> &BootInformationInner {
        unsafe { &*self.inner }
    }

    fn get_tag<'a>(&'a self, typ: u32) -> Option<&'a Tag> {
        self.tags().find(|tag| tag.typ == typ)
    }

    fn tags(&self) -> TagIter {
        TagIter::new(unsafe { self.inner.offset(1) } as *const _)
    }
}

impl BootInformationInner {
    fn has_valid_end_tag(&self) -> bool {
        const END_TAG: Tag = Tag { typ: 0, size: 8 };

        let self_ptr = self as *const _;
        let end_tag_addr = self_ptr as usize + (self.total_size - END_TAG.size) as usize;
        let end_tag = unsafe { &*(end_tag_addr as *const Tag) };

        end_tag.typ == END_TAG.typ && end_tag.size == END_TAG.size
    }
}

impl fmt::Debug for BootInformation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "multiboot information")?;

        writeln!(
            f,
            "S: {:#010X}, E: {:#010X}, L: {:#010X}",
            self.start_address(),
            self.end_address(),
            self.total_size()
        )?;

        if let Some(boot_loader_name_tag) = self.boot_loader_name_tag() {
            writeln!(f, "boot loader name: {}", boot_loader_name_tag.name())?;
        }

        if let Some(memory_map_tag) = self.memory_map_tag() {
            writeln!(f, "memory areas:")?;
            for area in memory_map_tag.memory_areas() {
                writeln!(
                    f,
                    "    S: {:#010X}, E: {:#010X}, L: {:#010X}",
                    area.start_address(),
                    area.end_address(),
                    area.size()
                )?;
            }
        }

        writeln!(f, "module tags:")?;
        for mt in self.module_tags() {
            writeln!(
                f,
                "    name: {:15}, s: {:#010x}, e: {:#010x}",
                mt.name(),
                mt.start_address(),
                mt.end_address()
            )?;
        }

        Ok(())
    }
}
