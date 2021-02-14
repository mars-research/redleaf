use pci_driver::DeviceBarRegions;
use console::println;

use crate::{VirtioNet, VirtioNetInner};

const VIRTIO_PCI_VID: u16 = 0x1af4;
const VIRTIO_PCI_DID: u16 = 0x1000;

pub(crate) struct PciFactory {
    mmio_base: Option<usize>,
}

impl pci_driver::PciDriver for PciFactory {
    fn probe(&mut self, bar_region: DeviceBarRegions) {
        println!("VirtioNet PCI probe called");
        match bar_region {
            DeviceBarRegions::Virtio(bar) => {
                unsafe {
                    self.mmio_base = Some(bar.get_base() as usize);
                }
            }
            ty => {
                println!("VirtioNet PCI probed with unsupported device {:?}", ty);
            }
        }
    }

    /// Returns the Vendor ID for a VIRTIO Network Device
    fn get_vid(&self) -> u16 {
        VIRTIO_PCI_VID
    }

    /// Returns the Device ID for a VIRTIO Network Device
    fn get_did(&self) -> u16 {
        // FIXME: Another possibility is the Transitional Device ID 0x1000
        VIRTIO_PCI_DID
    }

    fn get_driver_type(&self) -> pci_driver::PciDrivers {
        pci_driver::PciDrivers::VirtioDriver
    }
}

impl PciFactory {
    pub(crate) fn new() -> Self {
        Self {
            mmio_base: None,
        }
    }

    pub(crate) fn to_device(self) -> Option<VirtioNet> {
        self.mmio_base.map(|base| {
            let dev = unsafe { VirtioNetInner::new(base) };
            dev.to_shared()
        })
    }
}
