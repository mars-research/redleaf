// IDE
//
// A really sketchy IDE driver that makes use of the PIO mode with polling.
// Important caveats include that it uses CHS addressing and is still unsafe.
// The IRQ framework is so broken now so we're polling instead.
//
// References:
// - https://wiki.osdev.org/PCI_IDE_Controller
// - https://github.com/mit-pdos/xv6-public/blob/master/ide.c

use crate::redsys::resources::IOPort;

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
}

impl IDE {
    pub fn new() -> IDE {
        IDE {
        }
    }

    // HACK: Get an IOPort
    // In reality we want this to be managed centrally, controlled by some kind
    // of declarative capability specification. Or, at least we can refactor
    // IOPort to allow access to a range of ports.
    fn get_ioport(&self, port: u16) -> IOPort {
        unsafe { IOPort::new(port, true, true) }
    }

    /// Wait for the disk to become ready
    fn wait(&self) -> Result<(), ()> {
        // FIXME
        let port_1f7 = self.get_ioport(0x1f7);

        let mut r = port_1f7.inb().unwrap();
        while r & (IDE_BSY | IDE_DRDY) != IDE_DRDY {
            println!("{}", r);
            r = port_1f7.inb().unwrap();
        }

        if r & (IDE_DF | IDE_ERR) != 0 {
            return Err(());
        }

        Ok(())
    }

    /// Start a request
    fn start(&self, block: u32) {
        // Basically a translation of xv6's idestart
        // FIXME
        let port_3f6 = self.get_ioport(0x3f6);
        let port_1f2 = self.get_ioport(0x1f2);
        let port_1f3 = self.get_ioport(0x1f3);
        let port_1f4 = self.get_ioport(0x1f4);
        let port_1f5 = self.get_ioport(0x1f5);
        let port_1f6 = self.get_ioport(0x1f6);

        let sector_per_block = BSIZE / SECTOR_SIZE;
        let sector = block * sector_per_block;

        self.wait();

        port_3f6.outb(2); // No interrupts pls
        port_1f2.outb(sector_per_block as u8);

        port_1f3.outb((sector as u8) & 0xff);
        port_1f4.outb(((sector >> 8) as u8) & 0xff);
        port_1f5.outb(((sector >> 16) as u8) & 0xff);

        // FIXME: Specify disk #
        let disk: u8 = 0;
        port_1f6.outb(0xe0 | ((disk & 1) << 4) | ((sector >> 24) as u8) & 0x0f);
    }

    pub fn init(&self) {
        // FIXME
        let port_1f6 = self.get_ioport(0x1f6);

        self.wait().expect("IDE never became ready");

        // Use disk 0
        port_1f6.outb(0xe0 | (0 << 4)).unwrap();
    }

    /// Write a block into the disk
    pub fn write(&self, block: u32, data: &[u32; 512]) -> Result<(), ()> {
        // FIXME
        let port_1f0 = self.get_ioport(0x1f0);
        let port_1f7 = self.get_ioport(0x1f7);

        // Initiate request
        // FIXME: Use RDMUL and WRMUL when sector_per_block != 1
        self.start(block);
        port_1f7.outb(IDE_CMD_WRITE);
        port_1f0.outsl(data);

        // Wait for request to finish
        self.wait()
    }

    /// Read a block from the disk
    pub fn read(&self, block: u32, data: &mut [u32; 512]) -> Result<(), ()> {
        // FIXME
        let port_1f0 = self.get_ioport(0x1f0);
        let port_1f7 = self.get_ioport(0x1f7);

        // Initiate request
        // FIXME: Use RDMUL and WRMUL when sector_per_block != 1
        self.start(block);
        port_1f7.outb(IDE_CMD_READ);
        port_1f0.outsl(data);

        // Wait for request to finish
        self.wait();

        // Get data
        // FIXME: This is currently broken
        port_1f0.insl(data);
        Ok(())
    }
}
