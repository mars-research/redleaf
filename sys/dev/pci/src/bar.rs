use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PciBar {
    None,
    Memory(u32),
    Port(u16)
}

impl PciBar {
    pub fn is_none(&self) -> bool {
        match self {
            &PciBar::None => true,
            _ => false,
        }
    }
}

impl From<u32> for PciBar {
    fn from(bar: u32) -> Self {
        if bar & 0xFFFF_FFFC == 0 {
            PciBar::None
        } else if bar & 1 == 0 {
            PciBar::Memory(bar & 0xFFFF_FFF0)
        } else {
            PciBar::Port((bar & 0xFFFC) as u16)
        }
    }
}

impl fmt::Display for PciBar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &PciBar::Memory(address) => write!(f, "M {:>08X}", address),
            &PciBar::Port(address) => write!(f, "P {:>04X}", address),
            &PciBar::None => write!(f, "None")
        }
    }
}
