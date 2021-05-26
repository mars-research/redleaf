use console::println;
use pci_driver::DeviceBarRegions;

use crate::VirtioBlockInner;

use alloc::boxed::Box;
use interface::bdev::BDev;

const VIRTIO_PCI_VID: u16 = 0x1af4;
const VIRTIO_PCI_DID: u16 = 0x1001; // 0x1042

pub struct PciFactory {
    mmio_base: Option<usize>,
}

impl pci_driver::PciDriver for PciFactory {
    fn probe(&mut self, bar_region: DeviceBarRegions) {
        println!("VirtioBlock PCI probe called");
        match bar_region {
            DeviceBarRegions::Virtio(bar) => unsafe {
                self.mmio_base = Some(bar.get_base() as usize);
            },
            ty => {
                println!("VirtioBlock PCI probed with unsupported device {:?}", ty);
            }
        }
    }

    /// Returns the Vendor ID for a VIRTIO Block Device
    fn get_vid(&self) -> u16 {
        VIRTIO_PCI_VID
    }

    /// Returns the Device ID for a VIRTIO Block Device
    fn get_did(&self) -> u16 {
        // FIXME: Another possibility is the Transitional Device ID 0x1000
        VIRTIO_PCI_DID
    }

    fn get_driver_type(&self) -> pci_driver::PciDrivers {
        pci_driver::PciDrivers::VirtioDriver
    }
}

impl PciFactory {
    pub fn new() -> Self {
        Self { mmio_base: None }
    }

    pub fn to_device(self) -> Option<VirtioBlockInner> {
        self.mmio_base.map(|base| {
            let dev = unsafe { VirtioBlockInner::new(base) };
            dev
        })
    }
}
