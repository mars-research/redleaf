use super::Dma;
use libsyscalls::errors::Result;
use core::marker::Sized;

pub trait DmaAllocator
    where Self: Sized
{
    fn allocate() -> Result<Dma<Self>>;
}

#[macro_export]
macro_rules! zeroed_allocator {
    ($t: ty) => {
        impl DmaAllocator for $t {
            fn allocate() -> Result<Dma<Self>> {
                unsafe { Dma::zeroed() }
            }
        }
    };
}
