use super::{Dma, DmaAllocator};
use super::zeroed_allocator;
use libsyscalls::errors::Result;

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct NvmeCompletion {
    command_specific: u32,
    _rsvd: u32,
    pub sq_head: u16,
    pub sq_id: u16,
    pub cid: u16,
    pub status: u16,
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct NvmeCommand {
    /// Opcode
    pub opcode: u8,
    /// Flags
    pub flags: u8,
    /// Command ID
    pub cid: u16,
    /// Namespace identifier
    pub nsid: u32,
    /// Reserved
    pub _rsvd: u64,
    /// Metadata pointer
    pub mptr: u64,
    /// Data pointer
    pub dptr: [u64; 2],
    /// Command dword 10
    pub cdw10: u32,
    /// Command dword 11
    pub cdw11: u32,
    /// Command dword 12
    pub cdw12: u32,
    /// Command dword 13
    pub cdw13: u32,
    /// Command dword 14
    pub cdw14: u32,
    /// Command dword 15
    pub cdw15: u32,
}

zeroed_allocator!([NvmeCommand; 256]);
zeroed_allocator!([NvmeCompletion; 256]);

zeroed_allocator!([NvmeCommand; 512]);
zeroed_allocator!([NvmeCompletion; 512]);

zeroed_allocator!([NvmeCommand; 1024]);
zeroed_allocator!([NvmeCompletion; 1024]);

zeroed_allocator!([NvmeCommand; 8]);
zeroed_allocator!([NvmeCompletion; 8]);

zeroed_allocator!([NvmeCommand; 32]);
zeroed_allocator!([NvmeCompletion; 32]);

zeroed_allocator!([NvmeCommand; 16]);
zeroed_allocator!([NvmeCompletion; 16]);

zeroed_allocator!([u8; 4096]);
zeroed_allocator!([u32; 1024]);
zeroed_allocator!([u64; 512]);

pub fn allocate_dma<T>() -> Result<Dma<T>>
    where T: DmaAllocator
{
    T::allocate()
}
