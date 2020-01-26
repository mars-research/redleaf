use core::mem::size_of;
use core::ops::DerefMut;
use core::{ptr, u32};

use alloc::string::String;
use alloc::boxed::Box;
use alloc::sync::Arc;

use spin::{Mutex, MutexGuard};

use libdma::{Mmio, Dma};
use libdma::ahci::{HbaPrdtEntry, HbaCmdTable, HbaCmdHeader};
use libdma::ahci::allocate_dma;

use syscalls::errors::{Error, Result, EIO};
use libsyscalls::syscalls::sys_yield;
use libtime::get_rdtsc;

use ahci::{AhciBarRegion, AhciRegs, AhciArrayRegs, AhciPortRegs, AhciPortArrayRegs};

use crate::ahcid::disk_ata::{MAX_SECTORS_PER_PRDT_ENTRY, MAX_BYTES_PER_PRDT_ENTRY};

use console::{print, println};

// HBA reset
const HBA_GHC_HR: u32 = 1 << 0;
// AHCI enbale
const HBA_GHC_AE: u32 = 1 << 31; 

pub struct Hba {
    pub bar: Box<dyn AhciBarRegion>,
}

impl Hba {
    pub fn new(bar: Box<dyn AhciBarRegion>) -> Hba {
        Hba {
            bar: bar,
        }
    }

    fn reset(&self) {
        let bar = &self.bar;
        // Reset HBA
        bar.write_regf(AhciRegs::Ghc, HBA_GHC_HR, true);
        while bar.read_regf(AhciRegs::Ghc, HBA_GHC_HR)  {
            // spin
        }
    }

    pub fn init(&self) {
        let bar = &self.bar;
        bar.write_regf(AhciRegs::Ghc, HBA_GHC_AE, true);
        println!("   - AHCI CAP {:X} GHC {:X} IS {:X} PI {:X} VS {:X} CAP2 {:X} BOHC {:X}",
            bar.read_reg(AhciRegs::Cap), bar.read_reg(AhciRegs::Ghc), bar.read_reg(AhciRegs::Is), bar.read_reg(AhciRegs::Pi),
            bar.read_reg(AhciRegs::Vs), bar.read_reg(AhciRegs::Cap2), bar.read_reg(AhciRegs::Bohc)
        );
    }

    pub fn get_bar_ref(&self) -> &dyn AhciBarRegion {
        &*self.bar
    }
}

pub fn hba_port_dump(port: u64, bar: &Box<dyn AhciBarRegion>) {
    print!(
        "
        Is:{:08X}
        Ie:{:08X}
        Cmd:{:08X}
        Rsv0:{:08X}
        Tfd:{:08X}
        Sig:{:08X}
        Ssts:{:08X}
        Sctl:{:08X}
        Serr:{:08X}
        Sact:{:08X}
        Ci:{:08X}
        Sntf:{:08X}
        Fbs:{:08X}
        ",
        bar.read_port_reg(port, AhciPortRegs::Is),
        bar.read_port_reg(port, AhciPortRegs::Ie),
        bar.read_port_reg(port, AhciPortRegs::Cmd),
        bar.read_port_reg(port, AhciPortRegs::Rsv0),
        bar.read_port_reg(port, AhciPortRegs::Tfd),
        bar.read_port_reg(port, AhciPortRegs::Sig),
        bar.read_port_reg(port, AhciPortRegs::Ssts),
        bar.read_port_reg(port, AhciPortRegs::Sctl),
        bar.read_port_reg(port, AhciPortRegs::Serr),
        bar.read_port_reg(port, AhciPortRegs::Sact),
        bar.read_port_reg(port, AhciPortRegs::Ci),
        bar.read_port_reg(port, AhciPortRegs::Sntf),
        bar.read_port_reg(port, AhciPortRegs::Fbs),
    );
}
