use alloc::string::String;
use alloc::boxed::Box;
use alloc::sync::Arc;
use ahci::{AhciBarRegion, AhciRegs, AhciArrayRegs, AhciPortRegs, AhciPortArrayRegs};
use byteorder::{ByteOrder, LE};
use console::{print, println};
use core::mem::size_of;
use core::ops::DerefMut;
use core::{ptr, u32};
use syscalls::errors::{Error, Result, EIO};
use libdma::{Mmio, Dma};
use libdma::ahci::{HbaPrdtEntry, HbaCmdTable, HbaCmdHeader};
use libdma::ahci::allocate_dma;
use libsyscalls::syscalls::sys_yield;
use libtime::get_rdtsc;

use super::disk_ata::{MAX_SECTORS_PER_PRDT_ENTRY, MAX_BYTES_PER_PRDT_ENTRY};
use super::hba::{Hba, hba_port_dump};
use super::fis::{FisType, FisRegH2D};



const ATA_CMD_READ_DMA_EXT: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EXT: u8 = 0x35;
const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_PACKET: u8 = 0xA0;
const ATA_DEV_BUSY: u8 = 0x80;
const ATA_DEV_DRQ: u8 = 0x08;


// Perform initialization sequence.
const HBA_PORT_SCTL_DET_INIT: u32 = 0x1;

// Command List Running
const HBA_PORT_CMD_CR: u32 = 1 << 15;
// FIS Receive Running
const HBA_PORT_CMD_FR: u32 = 1 << 14;
const HBA_PORT_CMD_FRE: u32 = 1 << 4;
// Power on device
const HBA_PORT_CMD_POD: u32 = 1 << 2;
// Spin-up device
const HBA_PORT_CMD_SUD: u32 = 1 << 1;
// Start
const HBA_PORT_CMD_ST: u32 = 1 << 0;

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

pub struct HbaPort {
    hba: Arc<Hba>,
    port: u64,
    slotReady: [bool; 32],
}

impl HbaPort {
    pub fn new(hba: Arc<Hba>, port: u64) -> HbaPort {
        HbaPort {
            hba,
            port,
            slotReady: [true; 32],
        }
    }

    pub fn probe(&self) -> HbaPortType {
        if self.hba.bar.read_port_regf(self.port, AhciPortRegs::Ssts, HBA_SSTS_PRESENT) {
            let sig = self.hba.bar.read_port_reg(self.port, AhciPortRegs::Sig);
            let sig = match sig {
                HBA_SIG_ATA => HbaPortType::SATA,
                HBA_SIG_ATAPI => HbaPortType::SATAPI,
                HBA_SIG_PM => HbaPortType::PM,
                HBA_SIG_SEMB => HbaPortType::SEMB,
                _ => HbaPortType::Unknown(sig),
            };
            println!("AHCI drive found with type: {:?}", sig);
            sig
        } else {
            HbaPortType::None
        }
    }

    pub fn set_slot_ready(&mut self, slot: u32, ready: bool) {
        self.slotReady[slot as usize] = ready;
    }

    fn start(&self, hba: &Hba) {
        while self.hba.bar.read_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_CR) {
            // spin
        }

        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_FRE | HBA_PORT_CMD_ST, true);
    }

    // Stop command engine
    // See 10.1.2
    fn stop(&self, hba: &Hba) {
        // Clear ST (bit0)
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_ST, false);
        // Wait until FR CR (bit15) is cleared
        libtime::sys_ns_sleep(1_000_000_000);
        while (self.hba.bar.read_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_CR)) {
            // Spin
        }

        // Clear FRE
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_FRE, false);
        // Wait until FR (bit14) is cleared
        libtime::sys_ns_sleep(1_000_000_000);
        while (self.hba.bar.read_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_FR)) {
            // Spin
        }

        // TODO: If PxCMD.CR or PxCMD.FR do
        // not clear to ‘0’ correctly, then software may 
        // attempt a port reset or a full HBA reset to
        // recover.
    }

    fn slot(&self, hba: &Hba) -> Option<u32> {
        let slots = self.hba.bar.read_port_reg(self.port, AhciPortRegs::Sact) | self.hba.bar.read_port_reg(self.port, AhciPortRegs::Ci);

        for i in 0..32 {
            if slots & 1 << i == 0 && self.slotReady[i] {
                return Some(i as u32);
            }
        }
        None
    }

    // OS Dev equivelant: port_rebase
    // Read AHCI Spec r1.3.1 section 10.1.2
    pub fn init(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], fb: &mut Dma<[u8; 256]>) {
        // 3. Ensure that the controller is not in the running state by reading and examining each
        // implemented port’s PxCMD register. If PxCMD.ST, PxCMD.CR, PxCMD.FRE and
        // PxCMD.FR are all cleared, the port is in an idle state. Otherwise, the port is not idle and
        // should be placed in the idle state prior to manipulating HBA and port specific registers.
        // System software places a port into the idle state by clearing PxCMD.ST and waiting for
        // PxCMD.CR to return ‘0’ when read. Software should wait at least 500 milliseconds for
        // this to occur. If PxCMD.FRE is set to ‘1’, software should clear it to ‘0’ and wait at least
        // 500 milliseconds for PxCMD.FR to return ‘0’ when read. If PxCMD.CR or PxCMD.FR do
        // not clear to ‘0’ correctly, then software may attempt a port reset or a full HBA reset to
        // recover.
        self.stop(&self.hba);

        for i in 0..32 {
            let cmdheader = &mut clb[i];
            cmdheader.ctba.write(ctbas[i].physical() as u64);
            cmdheader.prdtl.write(0);
        }

        // 5. For each implemented port, system software shall allocate memory for and program:
        // PxCLB and PxCLBU (if CAP.S64A is set to ‘1’)
        // PxFB and PxFBU (if CAP.S64A is set to ‘1’)
        // It is good practice for system software to ‘zero-out’ the memory allocated and referenced
        // by PxCLB and PxFB. After setting PxFB and PxFBU to the physical address of the FIS
        // receive area, system software shall set PxCMD.FRE to ‘1’.
        // TODO: 64 bit address will overflow here
        self.hba.bar.write_port_reg_idx(self.port, AhciPortArrayRegs::Clb, 0, clb.physical() as u32);
        self.hba.bar.write_port_reg_idx(self.port, AhciPortArrayRegs::Clb, 1, (clb.physical() >> 32) as u32);
        self.hba.bar.write_port_reg_idx(self.port, AhciPortArrayRegs::Fb, 0, fb.physical() as u32);
        self.hba.bar.write_port_reg_idx(self.port, AhciPortArrayRegs::Fb, 1, (fb.physical() >> 32) as u32);
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_FRE, true);
        
        // Disable interrupt
        self.hba.bar.write_port_reg(self.port, AhciPortRegs::Ie, 0 /* TODO: Enable interrupts: 0b10111*/);
        
        // 6. For each implemented port, clear the PxSERR register, by writing ‘1s’ to each
        // implemented bit location.
        self.hba.bar.write_port_reg(self.port, AhciPortRegs::Serr, 0xFF_FF_FF_FF);
        
        // Disable power management
        const HBA_PORT_SCTL_IPM_DISABLE: u32 = 0x7 << 8;
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Sctl, HBA_PORT_SCTL_IPM_DISABLE, true);
        
        // Power on and spin up device
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_POD | HBA_PORT_CMD_SUD, true);
        
        println!("   - AHCI init {:X}", self.hba.bar.read_port_reg(self.port, AhciPortRegs::Cmd));
        hba_port_dump(self.port, &self.hba.bar);
    }

    // 10.4.2: COMRESET
    fn reset(&self, hba: &Hba) {
        // Prerequite
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_ST, false);
        // TODO: set timeout
        while self.hba.bar.read_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_CR) {
            // spin
        }

        // Actual reset
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Sctl, HBA_PORT_SCTL_DET_INIT, true);
        // Spin for one second
        libtime::sys_ns_sleep(1_000_000_000);
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Sctl, HBA_PORT_SCTL_DET_INIT, false);

        // Device presence detected and phy communication established.
        const HBA_PORT_SSTS_DET_EST: u32 = 0x3;
        while !self.hba.bar.read_port_regf(self.port, AhciPortRegs::Ssts, HBA_PORT_SSTS_DET_EST) {
            // Spin
        }
        
        const HBA_PROT_TFD_STS_BSY: u32 = 1 << 7;
        while !self.hba.bar.read_port_regf(self.port, AhciPortRegs::Tfd, HBA_PROT_TFD_STS_BSY) {
            // Spin
        }
        println!("HBA port is reset");
    }

    pub fn identify(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32]) -> Option<u64> {
        self.identify_inner(ATA_CMD_IDENTIFY, clb, ctbas)
    }

    // Shared between identify() and identify_packet()
    fn identify_inner(&mut self, cmd: u8, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32]) -> Option<u64> {
        let dest: Dma<[u16; 256]> = allocate_dma().unwrap();

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

        self.ata_stop(slot).ok()?;
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
    }

    pub fn ata_dma(&mut self, block: u64, sectors: u16, write: bool, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], buf: &[u8]) -> Option<u32> {
        println!("AHCI {} DMA BLOCK: {:X} SECTORS: {} WRITE: {}", self.port, block, sectors, write);
        if (sectors > 0xFFFF) {
            println!("Cannot R/W to more than {} sectors at a time", 0xFFFF);
            return None;
        }

        self.ata_start(clb, ctbas, |cmdheader, cmdfis, prdt_entries, _acmd| {
            if write {
                let cfl = cmdheader.cfl.read();
                const COMMAND_HEADER_DW0_W: u8 = 1 << 6;
                cmdheader.cfl.write(cfl | COMMAND_HEADER_DW0_W);
            }

            let chuncks = buf.chunks(MAX_BYTES_PER_PRDT_ENTRY);
            let num_chuncks = chuncks.len() as u16;
            for (chunck, mut prdt_entry) in chuncks.zip(prdt_entries.iter_mut()) {
                prdt_entry.dba.write(chunck.as_ptr() as u64);
                prdt_entry.dbc.write((chunck.len() as u32) - 1);
            }
            
            cmdheader.prdtl.write(num_chuncks);
            println!("The buffer is splitted into {} chuncks", num_chuncks);

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

            cmdfis.countl.write((sectors & 0xff) as u8);
            cmdfis.counth.write((sectors >> 8) as u8);
        })
    }

    pub fn ata_start<F>(&mut self, clb: &mut Dma<[HbaCmdHeader; 32]>, ctbas: &mut [Dma<HbaCmdTable>; 32], callback: F) -> Option<u32>
              where F: FnOnce(&mut HbaCmdHeader, &mut FisRegH2D, &mut [HbaPrdtEntry; 65536], &mut [Mmio<u8>; 16]) {
        //TODO: Should probably remove
        self.hba.bar.write_port_reg(self.port, AhciPortRegs::Is, u32::MAX);

        if let Some(slot) = self.slot(&self.hba) {
            {
                let cmdheader = &mut clb[slot as usize];
                let cfl = (size_of::<FisRegH2D>() / size_of::<u32>()) as u8;
                // CFL is 04:00
                assert!(cfl < 0b00011111);
                cmdheader.cfl.write(cfl);

                let cmdtbl = &mut ctbas[slot as usize];
                unsafe { ptr::write_bytes(cmdtbl.deref_mut() as *mut HbaCmdTable as *mut u8, 0, size_of::<HbaCmdTable>()); }

                let cmdfis = unsafe { &mut *(cmdtbl.cfis.as_mut_ptr() as *mut FisRegH2D) };
                cmdfis.fis_type.write(FisType::RegH2D as u8);

                let prdt_entry: &mut [HbaPrdtEntry; 65536] = unsafe { &mut *(&mut cmdtbl.prdt_entry as *mut _) };
                let acmd = unsafe { &mut *(&mut cmdtbl.acmd as *mut _) };

                callback(cmdheader, cmdfis, prdt_entry, acmd);
                // println!("{:?} {:?} {:?}", cmdheader, prdt_entry[0], cmdfis);
            }

            while self.hba.bar.read_port_regf(self.port, AhciPortRegs::Tfd, (ATA_DEV_BUSY | ATA_DEV_DRQ) as u32) {
                println!("Tfd: {:X}", self.hba.bar.read_port_reg(self.port, AhciPortRegs::Tfd));
            }

            self.hba.bar.write_port_regf(self.port, AhciPortRegs::Ci, 1 << slot, true);

            //TODO: Should probably remove
            self.start(&self.hba);

            Some(slot)
        } else {
            None
        }
    }

    pub fn ata_running(&self, slot: u32) -> bool {
        hba_port_dump(self.port, &self.hba.bar);
        (self.hba.bar.read_port_regf(self.port, AhciPortRegs::Ci, 1 << slot) || self.hba.bar.read_port_regf(self.port, AhciPortRegs::Tfd, 0x80)) && self.hba.bar.read_port_reg(self.port, AhciPortRegs::Is) & HBA_PORT_IS_ERR == 0
    }

    pub fn ata_stop(&mut self, slot: u32) -> Result<()> {
        while self.ata_running(slot) {
            // spin
            hba_port_dump(self.port, &self.hba.bar);
        }

        self.stop(&self.hba);

        if self.hba.bar.read_port_reg(self.port, AhciPortRegs::Is) & HBA_PORT_IS_ERR != 0 {
            // FIXME
            hba_port_dump(self.port, &self.hba.bar);
            
            // self.hba.bar.write_port_reg(self.port, AhciPortRegs::Is, u32::MAX);
            self.recover();
            Err(Error::new(EIO))
        } else {
            Ok(())
        }
    }

    // This could be complicated if we have multiple requests in flight
    pub fn recover(&mut self) {
        // Clears PxCMD.ST to ‘0’ to reset the PxCI register, waits for PxCMD.CR to clear to ‘0’
        self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_ST, false);
        while self.hba.bar.read_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_CR) {
            // Spin
        }
        // Clears any error bits in PxSERR to enable capturing new errors.
        self.hba.bar.write_port_reg(self.port, AhciPortRegs::Serr, 0);
        // Clears status bits in PxIS as appropriate
        self.hba.bar.write_port_reg(self.port, AhciPortRegs::Is, 0);
        // If PxTFD.STS.BSY or PxTFD.STS.DRQ is set to ‘1’, issue a COMRESET to the device to put
        // it in an idle state
        self.reset(&self.hba);
        // Sets PxCMD.ST to ‘1’ to enable issuing new commands
        // self.hba.bar.write_port_regf(self.port, AhciPortRegs::Cmd, HBA_PORT_CMD_ST, true);
        hba_port_dump(self.port, &self.hba.bar);
        self.hba.bar.write_port_reg(self.port, AhciPortRegs::Serr, 0xFF_FF_FF_FF);
    }
}
