

use core::{u32};


use alloc::boxed::Box;












use ahci::{AhciBarRegion, AhciRegs};



use console::{println};

// HBA reset
const HBA_GHC_HR: u32 = 1 << 0;
// AHCI enbale
const HBA_GHC_AE: u32 = 1 << 31; 

// BIOS busy
const HBA_BOHC_BB: u32 = 1 << 4;
// OS ownership change
const HBA_BOHC_OOC: u32 = 1 << 3;
// SMI on OS ownership change enabled
const HBA_BOHC_SOOE: u32 = 1 << 2;
// OS owned semaphore
const HBA_BOHC_OOS: u32 = 1 << 1;
// BIOS owned semaphore
const HBA_BOHC_BOS: u32 = 1 << 0;

pub struct Hba {
    pub bar: Box<dyn AhciBarRegion>,
}

impl Hba {
    pub fn new(bar: Box<dyn AhciBarRegion>) -> Hba {
        Hba {
            bar,
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

    pub fn request_ownership_from_bios(&self) {
        let bar = &self.bar;
        // AHCI r1.3.1 10.6.3
        // Request HBA ownership from BIOS
        println!("Requesting ownership from BIOS");
        bar.write_regf(AhciRegs::Bohc, HBA_BOHC_OOS, true);
        while bar.read_regf(AhciRegs::Bohc, HBA_BOHC_BOS) {
            // Spin
        }
        libtime::sys_ns_sleep(1_000_000_000);
        if bar.read_regf(AhciRegs::Bohc, HBA_BOHC_BB) {
            println!("BIOS still has outstanding requests. Wait for two more seconds");
            libtime::sys_ns_sleep(2_000_000_000);
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
