#![no_std]

extern crate alloc;

pub mod allocator;
pub mod ahci;
pub mod ixgbe;
mod dma;
mod mmio;

pub use allocator::DmaAllocator;
pub use dma::Dma;
pub use mmio::Mmio;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
