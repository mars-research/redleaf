use core::ptr;
use ahci::{AhciRegs, AhciArrayRegs, AhciBarRegion};
use syscalls::PciBar;
use alloc::boxed::Box;

macro_rules! reg_ahci {
    ($off: ident) => {
        Register {
            offset: AhciBar::$off,
        }
    }
}

macro_rules! reg_ahci_mult {
    ($off: ident, $num: expr, $mult: expr) => {
        ArrayRegister {
            offset: AhciBar::$off,
            num_regs: $num,
            multiplier: $mult
        }
    }
}

struct Register {
    offset: u64,
}

#[derive(Copy, Clone)]
struct ArrayRegister {
    offset: u64,
    num_regs: u64,
    multiplier: u64,
}

pub struct AhciBar {
    base: u64,
    size: usize,

    cap: Register,
    ghc: Register,
    is: Register,
    pi: Register,
    vs: Register,
    cccctl: Register,
    cccpts: Register,
    emloc: Register,
    emctl: Register,
    cap2: Register,
    bohc: Register,
    rsv: ArrayRegister,
    vendor: ArrayRegister,
}

impl AhciBar {
    const CAP: u64 = 0x00000;
    const GHC: u64 = 0x00004;
    const IS: u64 = 0x00008;
    const PI: u64 = 0x0000C;
    const VS: u64 = 0x00010;
    const CCCCTL: u64 = 0x00014;
    const CCCPTS: u64 = 0x00018;
    const EMLOC: u64 = 0x0001C;
    const EMCTL: u64 = 0x00020;
    const CAP2: u64 = 0x00024;
    const BOHC: u64 = 0x00028;

    const RSV: u64 = 0x0002C;
    const VENDOR: u64 = 0x000A0;
    const PORTS: u64 = 0x00100;

    pub fn new(base: u64, size: usize) -> AhciBar {
        AhciBar {
            base,
            size,
            cap: reg_ahci!(CAP),
            ghc: reg_ahci!(GHC),
            is: reg_ahci!(IS),
            pi: reg_ahci!(PI),
            vs: reg_ahci!(VS),
            cccctl: reg_ahci!(CCCCTL),
            cccpts: reg_ahci!(CCCPTS),
            emloc: reg_ahci!(EMLOC),
            emctl: reg_ahci!(EMCTL),
            cap2: reg_ahci!(CAP2),
            bohc: reg_ahci!(BOHC),
            rsv: reg_ahci_mult!(RSV, 116, 0x4),
            vendor: reg_ahci_mult!(VENDOR, 96, 0x4),
        }
    }
}

impl AhciBarRegion for AhciBar {
    fn get_base(&self) -> u64 {
        self.base
    }

    fn read_reg(&self, reg_enum: AhciRegs) -> u64 {
        let offset: u64;
        match reg_enum {
            AhciRegs::Cap => { offset = self.cap.offset },
            AhciRegs::Ghc => { offset = self.ghc.offset },
            AhciRegs::Is => { offset = self.is.offset },
            AhciRegs::Pi => { offset = self.pi.offset },
            AhciRegs::Vs => { offset = self.vs.offset },
            AhciRegs::Cccctl => { offset = self.cccctl.offset },
            AhciRegs::Cccpts => { offset = self.cccpts.offset },
            AhciRegs::Emloc => { offset = self.emloc.offset },
            AhciRegs::Emctl => { offset = self.emctl.offset },
            AhciRegs::Cap2 => { offset = self.cap2.offset },
            AhciRegs::Bohc => { offset = self.bohc.offset },
        }
        unsafe {
            ptr::read_volatile((self.base + offset) as *const u64)
        }
    }

    fn read_reg_idx(&self, reg_enum: AhciArrayRegs, idx: u64) -> u64 {
        let reg: ArrayRegister;
        match reg_enum {
            AhciArrayRegs::Rsv => { reg = self.rsv },
            AhciArrayRegs::Vendor => { reg = self.vendor },
        }

        if idx >= reg.num_regs {
            return 0;
        }
        unsafe {
            ptr::read_volatile((self.base + reg.offset + reg.multiplier * idx) as *const u64)
        }
    }

    fn write_reg(&self, reg_enum: AhciRegs, val: u64) {
        let offset: u64;
        match reg_enum {
            AhciRegs::Cap => { offset = self.cap.offset },
            AhciRegs::Ghc => { offset = self.ghc.offset },
            AhciRegs::Is => { offset = self.is.offset },
            AhciRegs::Pi => { offset = self.pi.offset },
            AhciRegs::Vs => { offset = self.vs.offset },
            AhciRegs::Cccctl => { offset = self.cccctl.offset },
            AhciRegs::Cccpts => { offset = self.cccpts.offset },
            AhciRegs::Emloc => { offset = self.emloc.offset },
            AhciRegs::Emctl => { offset = self.emctl.offset },
            AhciRegs::Cap2 => { offset = self.cap2.offset },
            AhciRegs::Bohc => { offset = self.bohc.offset },
        }
        unsafe {
            ptr::write_volatile((self.base + offset) as *mut u64, val);
        }
    }

    fn write_reg_idx(&self, reg_enum: AhciArrayRegs, idx: u64, val: u64) {
        let reg: ArrayRegister;
        match reg_enum {
            AhciArrayRegs::Rsv => { reg = self.rsv },
            AhciArrayRegs::Vendor => { reg = self.vendor },
        }

        if idx < reg.num_regs {
            unsafe {
                ptr::write_volatile((self.base + reg.offset + reg.multiplier * idx) as *mut u64, val)
            }
        }
    }
}
