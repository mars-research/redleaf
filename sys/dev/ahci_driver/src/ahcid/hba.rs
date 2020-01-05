use core::mem::size_of;
use core::ops::DerefMut;
use core::{ptr, u32};
use alloc::string::String;

use libdma::{Mmio, Dma};
use libdma::ahci::{HbaPrdtEntry, HbaCmdTable, HbaCmdHeader};

use libsyscalls::errors::{Error, Result, EIO};
use libsyscalls::syscalls::sys_yield;

use console::print;

use super::fis::{FisType, FisRegH2D};

const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_IDENTIFY_PACKET: u8 = 0xA1;
const ATA_CMD_PACKET: u8 = 0xA0;
const ATA_DEV_BUSY: u8 = 0x80;
const ATA_DEV_DRQ: u8 = 0x08;

const HBA_PORT_CMD_CR: u32 = 1 << 15;
const HBA_PORT_CMD_FR: u32 = 1 << 14;
const HBA_PORT_CMD_FRE: u32 = 1 << 4;
const HBA_PORT_CMD_ST: u32 = 1;
const HBA_PORT_IS_ERR: u32 = 1 << 30 | 1 << 29 | 1 << 28 | 1 << 27;
const HBA_SSTS_PRESENT: u32 = 0x3;
const HBA_SIG_ATA: u32 = 0x00000101;
const HBA_SIG_ATAPI: u32 = 0xEB140101;
const HBA_SIG_PM: u32 = 0x96690101;
const HBA_SIG_SEMB: u32 = 0xC33C0101;

#[derive(Debug)]
pub enum HbaPortType {
    None,
    Unknown(u32),
    SATA,
    SATAPI,
    PM,
    SEMB,
}

#[repr(packed)]
pub struct HbaPort {
    pub clb: [Mmio<u32>; 2], // 0x00, command list base address, 1K-byte aligned
    pub fb: [Mmio<u32>; 2], // 0x08, FIS base address, 256-byte aligned
    pub is: Mmio<u32>, // 0x10, interrupt status
    pub ie: Mmio<u32>, // 0x14, interrupt enable
    pub cmd: Mmio<u32>, // 0x18, command and status
    pub _rsv0: Mmio<u32>, // 0x1C, Reserved
    pub tfd: Mmio<u32>, // 0x20, task file data
    pub sig: Mmio<u32>, // 0x24, signature
    pub ssts: Mmio<u32>, // 0x28, SATA status (SCR0:SStatus)
    pub sctl: Mmio<u32>, // 0x2C, SATA control (SCR2:SControl)
    pub serr: Mmio<u32>, // 0x30, SATA error (SCR1:SError)
    pub sact: Mmio<u32>, // 0x34, SATA active (SCR3:SActive)
    pub ci: Mmio<u32>, // 0x38, command issue
    pub sntf: Mmio<u32>, // 0x3C, SATA notification (SCR4:SNotification)
    pub fbs: Mmio<u32>, // 0x40, FIS-based switch control
    pub _rsv1: [Mmio<u32>; 11], // 0x44 ~ 0x6F, Reserved
    pub vendor: [Mmio<u32>; 4], // 0x70 ~ 0x7F, vendor specific
}

impl HbaPort {
    pub fn probe(&self) -> HbaPortType {
        if self.ssts.readf(HBA_SSTS_PRESENT) {
            let sig = self.sig.read();
            match sig {
                HBA_SIG_ATA => HbaPortType::SATA,
                HBA_SIG_ATAPI => HbaPortType::SATAPI,
                HBA_SIG_PM => HbaPortType::PM,
                HBA_SIG_SEMB => HbaPortType::SEMB,
                _ => HbaPortType::Unknown(sig),
            }
        } else {
            HbaPortType::None
        }
    }

    pub fn start(&mut self) {
        while self.cmd.readf(HBA_PORT_CMD_CR) {
            sys_yield();
        }

        self.cmd.writef(HBA_PORT_CMD_FRE | HBA_PORT_CMD_ST, true);
    }

    pub fn stop(&mut self) {
        self.cmd.writef(HBA_PORT_CMD_ST, false);

        while self.cmd.readf(HBA_PORT_CMD_FR | HBA_PORT_CMD_CR) {
            sys_yield();
        }

        self.cmd.writef(HBA_PORT_CMD_FRE, false);
    }

    pub fn slot(&self) -> Option<u32> {
        let slots = self.sact.read() | self.ci.read();
        for i in 0..32 {
            if slots & 1 << i == 0 {
                return Some(i);
            }
        }
        None
    }

    pub fn init(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], fb: &mut Dma<[u8; 256]>) {
        self.stop();

        for i in 0..32 {
            let cmdheader = &mut clb[i];
            cmdheader.ctba.write(ctbas[i].physical() as u64);
            cmdheader.prdtl.write(0);
        }

        self.clb[0].write(clb.physical() as u32);
        self.clb[1].write((clb.physical() >> 32) as u32);
        self.fb[0].write(fb.physical() as u32);
        self.fb[1].write((fb.physical() >> 32) as u32);
        let is = self.is.read();
        self.is.write(is);
        self.ie.write(0 /*TODO: Enable interrupts: 0b10111*/);
        let serr = self.serr.read();
        self.serr.write(serr);

        // Disable power management
        let sctl = self.sctl.read() ;
        self.sctl.write(sctl | 7 << 8);

        // Power on and spin up device
        self.cmd.writef(1 << 2 | 1 << 1, true);

        print!("   - AHCI init {:X}\n", self.cmd.read());
    }

    pub unsafe fn identify(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32]) -> Option<u64> {
        self.identify_inner(ATA_CMD_IDENTIFY, clb, ctbas)
    }

    pub unsafe fn identify_packet(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32]) -> Option<u64> {
        self.identify_inner(ATA_CMD_IDENTIFY_PACKET, clb, ctbas)
    }

    // Shared between identify() and identify_packet()
    unsafe fn identify_inner(&mut self, cmd: u8, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32]) -> Option<u64> {
        let dest: Dma<[u16; 256]> = Dma::new([0; 256]).unwrap();

        let slot = self.ata_start(clb, ctbas, |cmdheader, cmdfis, prdt_entries, _acmd| {
            cmdheader.prdtl.write(1);

            let prdt_entry = &mut prdt_entries[0];
            prdt_entry.dba.write(dest.physical() as u64);
            prdt_entry.dbc.write(512 | 1);

            cmdfis.pm.write(1 << 7);
            cmdfis.command.write(cmd);
            cmdfis.device.write(0);
            cmdfis.countl.write(1);
            cmdfis.counth.write(0);
        })?;

        if self.ata_stop(slot).is_ok() {
            let mut serial = String::new();
            for word in 10..20 {
                let d = dest[word];
                let a = ((d >> 8) as u8) as char;
                if a != '\0' {
                    serial.push(a);
                }
                let b = (d as u8) as char;
                if b != '\0' {
                    serial.push(b);
                }
            }

            let mut firmware = String::new();
            for word in 23..27 {
                let d = dest[word];
                let a = ((d >> 8) as u8) as char;
                if a != '\0' {
                    firmware.push(a);
                }
                let b = (d as u8) as char;
                if b != '\0' {
                    firmware.push(b);
                }
            }

            let mut model = String::new();
            for word in 27..47 {
                let d = dest[word];
                let a = ((d >> 8) as u8) as char;
                if a != '\0' {
                    model.push(a);
                }
                let b = (d as u8) as char;
                if b != '\0' {
                    model.push(b);
                }
            }

            let mut sectors = (dest[100] as u64) |
                              ((dest[101] as u64) << 16) |
                              ((dest[102] as u64) << 32) |
                              ((dest[103] as u64) << 48);

            let lba_bits = if sectors == 0 {
                sectors = (dest[60] as u64) | ((dest[61] as u64) << 16);
                28
            } else {
                48
            };

            print!("   + Serial: {} Firmware: {} Model: {} {}-bit LBA Size: {} MB\n",
                        serial.trim(), firmware.trim(), model.trim(), lba_bits, sectors / 2048);

            Some(sectors * 512)
        } else {
            None
        }
    }

    pub fn ata_dma(&mut self, block: u64, sectors: usize, write: bool, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], buf: &mut Dma<[u8; 256 * 512]>) -> Option<u32> {
        // print!("{}", format!("AHCI {:X} DMA BLOCK: {:X} SECTORS: {} WRITE: {}\n", (self as *mut HbaPort) as usize, block, sectors, write));

        assert!(sectors > 0 && sectors < 256);

        self.ata_start(clb, ctbas, |cmdheader, cmdfis, prdt_entries, _acmd| {
            if write {
                let cfl = cmdheader.cfl.read();
                cmdheader.cfl.write(cfl | 1 << 7 | 1 << 6)
            }

            cmdheader.prdtl.write(1);

            let prdt_entry = &mut prdt_entries[0];
            prdt_entry.dba.write(buf.physical() as u64);
            prdt_entry.dbc.write(((sectors * 512) as u32) | 1);

            cmdfis.pm.write(1 << 7);
            if write {
                cmdfis.command.write(ATA_CMD_WRITE_DMA_EXT);
            } else {
                cmdfis.command.write(ATA_CMD_READ_DMA_EXT);
            }

            cmdfis.lba0.write(block as u8);
            cmdfis.lba1.write((block >> 8) as u8);
            cmdfis.lba2.write((block >> 16) as u8);

            cmdfis.device.write(1 << 6);

            cmdfis.lba3.write((block >> 24) as u8);
            cmdfis.lba4.write((block >> 32) as u8);
            cmdfis.lba5.write((block >> 40) as u8);

            cmdfis.countl.write(sectors as u8);
            cmdfis.counth.write((sectors >> 8) as u8);
        })
    }

    /// Send ATAPI packet
    pub fn atapi_dma(&mut self, cmd: &[u8; 16], size: u32, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], buf: &mut Dma<[u8; 256 * 512]>) -> Result<()> {
        let slot = self.ata_start(clb, ctbas, |cmdheader, cmdfis, prdt_entries, acmd| {
            let cfl = cmdheader.cfl.read();
            cmdheader.cfl.write(cfl | 1 << 5);

            cmdheader.prdtl.write(1);

            let prdt_entry = &mut prdt_entries[0];
            prdt_entry.dba.write(buf.physical() as u64);
            prdt_entry.dbc.write(size - 1);

            cmdfis.pm.write(1 << 7);
            cmdfis.command.write(ATA_CMD_PACKET);
            cmdfis.device.write(0);
            cmdfis.lba1.write(0);
            cmdfis.lba2.write(0);
            cmdfis.featurel.write(1);
            cmdfis.featureh.write(0);

            unsafe { ptr::write_volatile(acmd.as_mut_ptr() as *mut [u8; 16], *cmd) };
        }).ok_or(Error::new(EIO))?;
        self.ata_stop(slot)
    }

    pub fn ata_start<F>(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], callback: F) -> Option<u32>
              where F: FnOnce(&mut HbaCmdHeader, &mut FisRegH2D, &mut [HbaPrdtEntry; 65536], &mut [Mmio<u8>; 16]) {

        //TODO: Should probably remove
        self.is.write(u32::MAX);

        if let Some(slot) = self.slot() {
            {
                let cmdheader = &mut clb[slot as usize];
                cmdheader.cfl.write((size_of::<FisRegH2D>() / size_of::<u32>()) as u8);

                let cmdtbl = &mut ctbas[slot as usize];
                unsafe { ptr::write_bytes(cmdtbl.deref_mut() as *mut HbaCmdTable as *mut u8, 0, size_of::<HbaCmdTable>()); }

                let cmdfis = unsafe { &mut *(cmdtbl.cfis.as_mut_ptr() as *mut FisRegH2D) };
                cmdfis.fis_type.write(FisType::RegH2D as u8);

                let prdt_entry = unsafe { &mut *(&mut cmdtbl.prdt_entry as *mut _) };
                let acmd = unsafe { &mut *(&mut cmdtbl.acmd as *mut _) };

                callback(cmdheader, cmdfis, prdt_entry, acmd)
            }

            while self.tfd.readf((ATA_DEV_BUSY | ATA_DEV_DRQ) as u32) {
                sys_yield();
            }

            self.ci.writef(1 << slot, true);

            //TODO: Should probably remove
            self.start();

            Some(slot)
        } else {
            None
        }
    }

    pub fn ata_running(&self, slot: u32) -> bool {
        (self.ci.readf(1 << slot) || self.tfd.readf(0x80)) && self.is.read() & HBA_PORT_IS_ERR == 0
    }

    pub fn ata_stop(&mut self, slot: u32) -> Result<()> {
        while self.ata_running(slot) {
            sys_yield();
        }

        self.stop();

        if self.is.read() & HBA_PORT_IS_ERR != 0 {
            print!("ERROR IS {:X} IE {:X} CMD {:X} TFD {:X}\nSSTS {:X} SCTL {:X} SERR {:X} SACT {:X}\nCI {:X} SNTF {:X} FBS {:X}\n",
                    self.is.read(), self.ie.read(), self.cmd.read(), self.tfd.read(),
                    self.ssts.read(), self.sctl.read(), self.serr.read(), self.sact.read(),
                    self.ci.read(), self.sntf.read(), self.fbs.read());
            self.is.write(u32::MAX);
            Err(Error::new(EIO))
        } else {
            Ok(())
        }
    }
}

#[repr(packed)]
pub struct HbaMem {
    pub cap: Mmio<u32>, // 0x00, Host capability
    pub ghc: Mmio<u32>, // 0x04, Global host control
    pub is: Mmio<u32>, // 0x08, Interrupt status
    pub pi: Mmio<u32>, // 0x0C, Port implemented
    pub vs: Mmio<u32>, // 0x10, Version
    pub ccc_ctl: Mmio<u32>, // 0x14, Command completion coalescing control
    pub ccc_pts: Mmio<u32>, // 0x18, Command completion coalescing ports
    pub em_loc: Mmio<u32>, // 0x1C, Enclosure management location
    pub em_ctl: Mmio<u32>, // 0x20, Enclosure management control
    pub cap2: Mmio<u32>, // 0x24, Host capabilities extended
    pub bohc: Mmio<u32>, // 0x28, BIOS/OS handoff control and status
    pub _rsv: [Mmio<u8>; 116], // 0x2C - 0x9F, Reserved
    pub vendor: [Mmio<u8>; 96], // 0xA0 - 0xFF, Vendor specific registers
    pub ports: [HbaPort; 32], // 0x100 - 0x10FF, Port control registers
}

impl HbaMem {
    pub fn init(&mut self) {
        /*
        self.ghc.writef(1, true);
        while self.ghc.readf(1) {
            pause();
        }
        */
        self.ghc.write(1 << 31 | 1 << 1);

        print!("   - AHCI CAP {:X} GHC {:X} IS {:X} PI {:X} VS {:X} CAP2 {:X} BOHC {:X}",
            self.cap.read(), self.ghc.read(), self.is.read(), self.pi.read(),
            self.vs.read(), self.cap2.read(), self.bohc.read());
    }
}
