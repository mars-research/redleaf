// ATA PIO Mode Device
//
// References:
// - https://wiki.osdev.org/ATA_PIO_Mode

// TODO: Move redsys to a separate crate
use super::super::resources::IOPort;

pub struct ATAPIODevice {
    // Offse 0
    pub data: IOPort,

    // I/O Offset 1
    pub error: IOPort,
    pub features: IOPort,

    // I/O Offset 2
    pub sectorCount: IOPort,

    // I/O Offset 3 ~ 5
    pub lbaLo: IOPort,
    pub lbaMid: IOPort,
    pub lbaHi: IOPort,

    // I/O Offset 6
    pub drive: IOPort,

    // I/O Offset 7
    pub status: IOPort,
    pub command: IOPort,

    // Control Offset 0
    pub altStatus: IOPort,
    pub control: IOPort,

    // Control Offset 1
    pub driveAddress: IOPort,
}

impl ATAPIODevice {
    pub unsafe fn new(ioBase: u16, controlBase: u16) -> ATAPIODevice {
        ATAPIODevice {
            // I/O Offset 0
            data: IOPort::new(ioBase, true, true),

            // I/O Offset 1
            error: IOPort::new(ioBase + 1, true, false),
            features: IOPort::new(ioBase + 1, false, true),

            // I/O Offset 2
            sectorCount: IOPort::new(ioBase + 2, true, true),

            // I/O Offset 3 ~ 5
            lbaLo: IOPort::new(ioBase + 3, true, true),
            lbaMid: IOPort::new(ioBase + 4, true, true),
            lbaHi: IOPort::new(ioBase + 5, true, true),

            // I/O Offset 6
            drive: IOPort::new(ioBase + 6, true, true),

            // I/O Offset 7
            status: IOPort::new(ioBase + 7, true, false),
            command: IOPort::new(ioBase + 7, false, true),

            // Control Offset 0
            altStatus: IOPort::new(controlBase, true, false),
            control: IOPort::new(controlBase, false, true),

            // Control Offset 1
            driveAddress: IOPort::new(controlBase + 1, true, false),
        }
    }

    pub unsafe fn primary() -> ATAPIODevice {
        Self::new(0x1f0, 0x3f6)
    }

    pub unsafe fn secondary() -> ATAPIODevice {
        Self::new(0x170, 0x376)
    }
}
