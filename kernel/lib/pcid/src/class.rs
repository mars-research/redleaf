#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PciClass {
    Legacy,
    Storage,
    Network,
    Display,
    Multimedia,
    Memory,
    Bridge,
    SimpleComms,
    Peripheral,
    Input,
    Docking,
    Processor,
    SerialBus,
    Wireless,
    IntelligentIo,
    SatelliteComms,
    Cryptography,
    SignalProc,
    Reserved(u8),
    Unknown
}

impl From<u8> for PciClass {
    fn from(class: u8) -> PciClass {
        match class {
            0x00 => PciClass::Legacy,
            0x01 => PciClass::Storage,
            0x02 => PciClass::Network,
            0x03 => PciClass::Display,
            0x04 => PciClass::Multimedia,
            0x05 => PciClass::Memory,
            0x06 => PciClass::Bridge,
            0x07 => PciClass::SimpleComms,
            0x08 => PciClass::Peripheral,
            0x09 => PciClass::Input,
            0x0A => PciClass::Docking,
            0x0B => PciClass::Processor,
            0x0C => PciClass::SerialBus,
            0x0D => PciClass::Wireless,
            0x0E => PciClass::IntelligentIo,
            0x0F => PciClass::SatelliteComms,
            0x10 => PciClass::Cryptography,
            0x11 => PciClass::SignalProc,
            0xFF => PciClass::Unknown,
            reserved => PciClass::Reserved(reserved)
        }
    }
}

impl Into<u8> for PciClass {
    fn into(self) -> u8 {
        match self {
            PciClass::Legacy => 0x00,
            PciClass::Storage => 0x01,
            PciClass::Network => 0x02,
            PciClass::Display => 0x03,
            PciClass::Multimedia => 0x04,
            PciClass::Memory => 0x05,
            PciClass::Bridge => 0x06,
            PciClass::SimpleComms => 0x07,
            PciClass::Peripheral => 0x08,
            PciClass::Input => 0x09,
            PciClass::Docking => 0x0A,
            PciClass::Processor => 0x0B,
            PciClass::SerialBus => 0x0C,
            PciClass::Wireless => 0x0D,
            PciClass::IntelligentIo => 0x0E,
            PciClass::SatelliteComms => 0x0F,
            PciClass::Cryptography => 0x10,
            PciClass::SignalProc => 0x11,
            PciClass::Unknown => 0xFF,
            PciClass::Reserved(reserved) => reserved
        }
    }
}
