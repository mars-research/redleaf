/// RedLeaf block device interface
use syscalls::errors::Result;
use alloc::boxed::Box;

pub trait BDev {
    fn read(&self, block: u32, data: &mut [u8; 512]);
    fn write(&self, block: u32, data: &[u8; 512]);

    fn read_contig(&self, block: u32, data: &mut [u8]);

    fn submit(&self, block: u64, write: bool, buf: Box<[u8]>) -> Result<u32>;
    fn poll(&self, slot: u32) -> Result<Option<Box<[u8]>>>;
}
pub type BDevPtr = Box<dyn BDev + Send + Sync>;

