// IDE
//
// A really sketchy IDE driver that makes use of the PIO mode.
//
// References:
// - https://wiki.osdev.org/PCI_IDE_Controller
// - https://github.com/mit-pdos/xv6-public/blob/master/ide.c

use crate::redsys::resources::IOPort;
use crate::redsys::devices::ATAPIODevice;
use alloc::sync::Arc;
use spin::Mutex;

const BSIZE: u32 = 512;
const SECTOR_SIZE: u32 = 512;
const IDE_BSY: u8 = 0x80;
const IDE_DRDY: u8 = 0x40;
const IDE_DF: u8 = 0x20;
const IDE_ERR: u8 = 0x01;

const IDE_CMD_READ: u8 = 0x20;
const IDE_CMD_WRITE: u8 = 0x30;
const IDE_CMD_RDMUL: u8 = 0xc4;
const IDE_CMD_WRMUL: u8 = 0xc5;

pub struct IDE {
    _device: Arc<Mutex<ATAPIODevice>>,
}

impl IDE {
    pub fn new(ataPioDevice: Arc<Mutex<ATAPIODevice>>) -> IDE {
        IDE {
            _device: ataPioDevice,
        }
    }

    /// Wait for the disk to become ready
    fn wait(&self, device: &mut ATAPIODevice) -> Result<(), ()> {
        let mut r = device.status.inb().unwrap();
        while r & (IDE_BSY | IDE_DRDY) != IDE_DRDY {
            r = device.status.inb().unwrap();
        }

        if r & (IDE_DF | IDE_ERR) != 0 {
            return Err(());
        }

        Ok(())
    }

    /// Start a request
    fn start(&self, device: &mut ATAPIODevice, block: u32) {
        // Basically a translation of xv6's idestart

        let sector_per_block = BSIZE / SECTOR_SIZE;
        let sector = block * sector_per_block;

        self.wait(&mut *device);

        device.control.outb(2); // No interrupts pls
        device.sectorCount.outb(sector_per_block as u8);

        device.lbaLo.outb((sector as u8) & 0xff);
        device.lbaMid.outb(((sector >> 8) as u8) & 0xff);
        device.lbaHi.outb(((sector >> 16) as u8) & 0xff);

        // FIXME: Specify disk #
        let disk: u8 = 0;
        device.drive.outb(0xe0 | ((disk & 1) << 4) | ((sector >> 24) as u8) & 0x0f);
    }

    pub fn init(&self) {
        let mut device = self._device.lock();

        self.wait(&mut *device).expect("IDE never became ready");

        // Use disk 0
        device.drive.outb(0xe0 | (0 << 4)).unwrap();
    }

    /// Write a block into the disk
    pub fn write(&self, block: u32, data: &[u32; 512]) -> Result<(), ()> {
        let mut device = self._device.lock();

        // Initiate request
        // FIXME: Use RDMUL and WRMUL when sector_per_block != 1
        self.start(&mut *device, block);
        device.command.outb(IDE_CMD_WRITE);
        device.data.outsl(data);

        // Wait for request to finish
        self.wait(&mut *device)
    }

    /// Read a block from the disk
    pub fn read(&self, block: u32, data: &mut [u32; 512]) -> Result<(), ()> {
        let mut device = self._device.lock();

        // Initiate request
        // FIXME: Use RDMUL and WRMUL when sector_per_block != 1
        self.start(&mut *device, block);
        device.command.outb(IDE_CMD_READ);
        device.data.outsl(data);

        // Wait for request to finish
        self.wait(&mut *device);

        // Get data
        // FIXME: This is currently broken
        device.data.insl(data);
        Ok(())
    }
}
