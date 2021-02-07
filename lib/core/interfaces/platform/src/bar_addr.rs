#[derive(Copy, Clone, Debug)]
pub struct PciBarAddr {
    base: u32,
    size: usize,
}

impl PartialEq for PciBarAddr {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl PciBarAddr {
    pub unsafe fn new(base: u32, size: usize) -> PciBarAddr {
        PciBarAddr{
            base,
            size,
        }
    }

    pub unsafe fn get_base(&self) -> u32 {
        self.base
    }

    pub unsafe fn get_size(&self) -> usize {
        self.size
    }
}
