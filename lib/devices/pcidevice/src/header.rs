
use alloc::vec::Vec;
use byteorder::{LittleEndian, ByteOrder};
use pci_driver::PciClass;
use console::println;
use platform::PciBarAddr;
use super::{pci_read_bars, pci_read_range, pci_read, pci_write, PciAddress};
use libsyscalls::errors::Result;
use libsyscalls::errors::{Error, EINVAL, ENODEV};

bitflags! {
    /// Flags found in the status register of a PCI device
    pub struct PciHeaderType: u8 {
        /// A general PCI device (Type 0x01).
        const GENERAL       = 0b0000_0000;
        /// A PCI-to-PCI bridge device (Type 0x01).
        const PCITOPCI      = 0b0000_0001;
        /// A PCI-to-PCI bridge device (Type 0x02).
        const CARDBUSBRIDGE = 0b0000_0010;
        /// A multifunction device.
        const MULTIFUNCTION = 0b0100_0000;
        /// Mask used for fetching the header type.
        const HEADER_TYPE   = 0b0000_0011;
    }
}

pub struct PciDeviceHeader {
    hdr: PciHeader,
}

impl PciDeviceHeader {
    fn new(hdr: PciHeader) -> PciDeviceHeader {
        PciDeviceHeader {
            hdr,
        }
    }

    pub fn get_bar(&self, idx: usize) -> Option<&PciBarAddr> {
        self.hdr.get_bar(idx)
    }

    pub fn class(&self) -> PciClass {
        self.hdr.class()
    }

    pub fn subclass(&self) -> u8 {
        self.hdr.subclass()
    }

    pub fn vendor_id(&self) -> u16 {
        self.hdr.vendor_id()
    }

    pub fn device_id(&self) -> u16 {
        self.hdr.device_id()
    }

}

enum PciHeader {
    General {
        vendor_id: u16,
        device_id: u16,
        command: u16,
        status: u16,
        revision: u8,
        interface: u8,
        subclass: u8,
        class: PciClass,
        cache_line_size: u8,
        latency_timer: u8,
        header_type: PciHeaderType,
        bist: u8,
        bars: Vec<Option<PciBarAddr>>,
        cardbus_cis_ptr: u32,
        subsystem_vendor_id: u16,
        subsystem_id: u16,
        expansion_rom_bar: u32,
        cap_pointer: u8,
        interrupt_line: u8,
        interrupt_pin: u8,
        min_grant: u8,
        max_latency: u8
    },
    PciToPci {
        vendor_id: u16,
        device_id: u16,
        command: u16,
        status: u16,
        revision: u8,
        interface: u8,
        subclass: u8,
        class: PciClass,
        cache_line_size: u8,
        latency_timer: u8,
        header_type: PciHeaderType,
        bist: u8,
        bars: Vec<Option<PciBarAddr>>,
        primary_bus_num: u8,
        secondary_bus_num: u8,
        subordinate_bus_num: u8,
        secondary_latency_timer: u8,
        io_base: u8,
        io_limit: u8,
        secondary_status: u16,
        mem_base: u16,
        mem_limit: u16,
        prefetch_base: u16,
        prefetch_limit: u16,
        prefetch_base_upper: u32,
        prefetch_limit_upper: u32,
        io_base_upper: u16,
        io_limit_upper: u16,
        cap_pointer: u8,
        expansion_rom: u32,
        interrupt_line: u8,
        interrupt_pin : u8,
        bridge_control: u16
    }
}

pub fn pci_enable_bus_mastering(pci_addr: &PciAddress) {
    let cmd_status = pci_read(pci_addr, 4);
    let mut command = cmd_status & 0xFFFF;
    let status = (cmd_status >> 16) & 0xFFFF;
    command |= 1 << 2;
    let value = u32::from(command) | (u32::from(status) << 16);
    pci_write(pci_addr, 4, value);
    println!("Enable bus mastering for device");
}

/// Parse the bytes found in the Configuration Space of the PCI device into
/// a more usable PciHeader.
pub fn read_config_space(pci_addr: &PciAddress) -> Result<PciDeviceHeader> {
    if pci_read(&pci_addr, 0) != 0xffff_ffff {
        // Read the initial 16 bytes and set variables used by all header types.
        let mut bytes = pci_read_range(&pci_addr, 0, 16);
        let vendor_id = LittleEndian::read_u16(&bytes[0..2]);
        let device_id = LittleEndian::read_u16(&bytes[2..4]);
        let command = LittleEndian::read_u16(&bytes[4..6]);
        let status = LittleEndian::read_u16(&bytes[6..8]);

        println!("command {:x} status {:x}", command, status);

        // TODO: Move this into a trait object from where a driver can invoke this
        pci_enable_bus_mastering(pci_addr);
        let revision = bytes[8];
        let interface = bytes[9];
        let subclass = bytes[10];
        let class = PciClass::from(bytes[11]);
        let cache_line_size = bytes[12];
        let latency_timer = bytes[13];
        let header_type = PciHeaderType::from_bits_truncate(bytes[14]);
        let bist = bytes[15];
        match header_type & PciHeaderType::HEADER_TYPE {
            PciHeaderType::GENERAL => {
                let bars = pci_read_bars(pci_addr, PciHeaderType::GENERAL); 

                #[cfg(feature = "c220g2_ixgbe")]
                {
                    // Cloudlab has dual port ixgbe devices and the we need to attach our driver
                    // to the second device.
                    if bars[0] == PciBarAddr::new(0xc7900000, 0) {
                        return Err(Error::new(ENODEV));
                    }
                }

                let bytes = pci_read_range(&pci_addr, 30, 24);
                let cardbus_cis_ptr = LittleEndian::read_u32(&bytes[0..4]);
                let subsystem_vendor_id = LittleEndian::read_u16(&bytes[4..6]);
                let subsystem_id = LittleEndian::read_u16(&bytes[6..8]);
                let expansion_rom_bar = LittleEndian::read_u32(&bytes[8..12]);
                // TODO: Parse out the capabilities list.
                let cap_pointer = bytes[12];
                let interrupt_line = bytes[20];
                let interrupt_pin = bytes[21];
                let min_grant = bytes[22];
                let max_latency = bytes[23];
                Ok(PciDeviceHeader::new(PciHeader::General {
                    vendor_id, device_id, command, status, revision, interface,
                    subclass, class, cache_line_size, latency_timer, header_type,
                    bist, bars, cardbus_cis_ptr, subsystem_vendor_id, subsystem_id,
                    expansion_rom_bar, cap_pointer, interrupt_line, interrupt_pin,
                    min_grant, max_latency
                }))
            },
            PciHeaderType::PCITOPCI => {
                let bars = pci_read_bars(pci_addr, PciHeaderType::PCITOPCI); 

                let bytes = pci_read_range(&pci_addr, 24, 40);
                let primary_bus_num = bytes[0];
                let secondary_bus_num = bytes[1];
                let subordinate_bus_num = bytes[2];
                let secondary_latency_timer = bytes[3];
                let io_base = bytes[4];
                let io_limit = bytes[5];
                let secondary_status = LittleEndian::read_u16(&bytes[6..8]);
                let mem_base = LittleEndian::read_u16(&bytes[8..10]);
                let mem_limit = LittleEndian::read_u16(&bytes[10..12]);
                let prefetch_base = LittleEndian::read_u16(&bytes[12..14]);
                let prefetch_limit = LittleEndian::read_u16(&bytes[14..16]);
                let prefetch_base_upper = LittleEndian::read_u32(&bytes[16..20]);
                let prefetch_limit_upper = LittleEndian::read_u32(&bytes[20..24]);
                let io_base_upper = LittleEndian::read_u16(&bytes[24..26]);
                let io_limit_upper = LittleEndian::read_u16(&bytes[26..28]);
                // TODO: Parse out the capabilities list.
                let cap_pointer = bytes[28];
                let expansion_rom = LittleEndian::read_u32(&bytes[32..36]);
                let interrupt_line = bytes[36];
                let interrupt_pin = bytes[37];
                let bridge_control = LittleEndian::read_u16(&bytes[38..40]);
                Ok(PciDeviceHeader::new(PciHeader::PciToPci {
                    vendor_id, device_id, command, status, revision, interface,
                    subclass, class, cache_line_size, latency_timer, header_type,
                    bist, bars, primary_bus_num, secondary_bus_num, subordinate_bus_num,
                    secondary_latency_timer, io_base, io_limit, secondary_status,
                    mem_base, mem_limit, prefetch_base, prefetch_limit, prefetch_base_upper,
                    prefetch_limit_upper, io_base_upper, io_limit_upper, cap_pointer,
                    expansion_rom, interrupt_line, interrupt_pin, bridge_control
                }))
            },
            id => Err(Error::new(EINVAL))
        }
    } else {
        Err(Error::new(ENODEV))
    }
}

impl PciHeader {
    /// Return the Header Type.
    pub fn header_type(&self) -> PciHeaderType {
        match self {
            &PciHeader::General { header_type, .. } | &PciHeader::PciToPci { header_type, .. } => header_type,
        }
    }

    /// Return the Vendor ID field.
    pub fn vendor_id(&self) -> u16 {
        match self {
            &PciHeader::General { vendor_id, .. } | &PciHeader::PciToPci { vendor_id, .. } => vendor_id,
        }
    }

    /// Return the Device ID field.
    pub fn device_id(&self) -> u16 {
        match self {
            &PciHeader::General { device_id, .. } | &PciHeader::PciToPci { device_id, .. } => device_id,
        }
    }

    /// Return the Command field.
    pub fn command(&self) -> u16 {
        match self {
            &PciHeader::General { command, .. } | &PciHeader::PciToPci { command, .. } => command,
        }
    }

    /// Return the status field.
    pub fn status(&self) -> u16 {
        match self {
            &PciHeader::General { status, .. } | &PciHeader::PciToPci { status, .. } => status,
        }
    }

    /// Return the Revision field.
    pub fn revision(&self) -> u8 {
        match self {
            &PciHeader::General { revision, .. } | &PciHeader::PciToPci { revision, .. } => revision,
        }
    }

    /// Return the Interface field.
    pub fn interface(&self) -> u8 {
        match self {
            &PciHeader::General { interface, .. } | &PciHeader::PciToPci { interface, .. } => interface,
        }
    }

    /// Return the Subclass field.
    pub fn subclass(&self) -> u8 {
        match self {
            &PciHeader::General { subclass, .. } | &PciHeader::PciToPci { subclass, .. } => subclass,
        }
    }

    /// Return the Class field.
    pub fn class(&self) -> PciClass {
        match self {
            &PciHeader::General { class, .. } | &PciHeader::PciToPci { class, .. } => class,
        }
    }

    /// Return the Headers BARs.
    pub fn bars(&self) -> &Vec<Option<PciBarAddr>> {
        match self {
            &PciHeader::General { ref bars, .. } => bars,
            &PciHeader::PciToPci { ref bars, .. } => bars,
        }
    }

    /// Return the BAR at the given index.
    ///
    /// # Panics
    /// This function panics if the requested BAR index is beyond the length of the header
    /// types BAR array.
    ///
    pub fn get_bar(&self, idx: usize) -> Option<&PciBarAddr> {
        match self {
            PciHeader::General { bars, .. } => {
                assert!(idx < 6, "the general PCI device only has 6 BARs");
                match &bars[idx] {
                    Some(bar) => Some(bar),
                    _ => None,
                }
            },
            PciHeader::PciToPci { bars, .. } => {
                assert!(idx < 2, "the general PCI device only has 2 BARs");
                match &bars[idx] {
                    Some(bar) => Some(bar),
                    _ => None,
                }
            }
        }
    }

    /// Return the Interrupt Line field.
    pub fn interrupt_line(&self) -> u8 {
        match self {
            &PciHeader::General { interrupt_line, .. } | &PciHeader::PciToPci { interrupt_line, .. } =>
                interrupt_line,
        }
    }

}
