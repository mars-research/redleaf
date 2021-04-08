#![no_std]

use ahci_regs::{AhciArrayRegs, AhciBarRegion, AhciPortArrayRegs, AhciPortRegs, AhciRegs};
use core::ptr;

macro_rules! reg_ahci {
    ($off: ident) => {
        Register {
            offset: AhciBar::$off,
        }
    };
}

macro_rules! reg_ahci_mult {
    ($off: ident, $num: expr, $mult: expr) => {
        ArrayRegister {
            offset: AhciBar::$off,
            num_regs: $num,
            multiplier: $mult,
        }
    };
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

    const PORT_CLB: u64 = 0x00;
    const PORT_FB: u64 = 0x08;
    const PORT_IS: u64 = 0x10;
    const PORT_IE: u64 = 0x14;
    const PORT_CMD: u64 = 0x18;
    const PORT_RSV0: u64 = 0x1C;
    const PORT_TFD: u64 = 0x20;
    const PORT_SIG: u64 = 0x24;
    const PORT_SSTS: u64 = 0x28;
    const PORT_SCTL: u64 = 0x2C;
    const PORT_SERR: u64 = 0x30;
    const PORT_SACT: u64 = 0x34;
    const PORT_CI: u64 = 0x38;
    const PORT_SNTF: u64 = 0x3C;
    const PORT_FBS: u64 = 0x40;
    const PORT_RSV1: u64 = 0x44;
    const PORT_VENDOR: u64 = 0x70;

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

    fn get_port_reg_offset(&self, port: u64, reg_enum: AhciPortRegs) -> u64 {
        assert!(port < 32);

        // 0x80 = 128
        let port_offset = self.base + Self::PORTS + 128 * port;

        let reg_offset = match reg_enum {
            AhciPortRegs::Is => Self::PORT_IS,
            AhciPortRegs::Ie => Self::PORT_IE,
            AhciPortRegs::Cmd => Self::PORT_CMD,
            AhciPortRegs::Rsv0 => Self::PORT_RSV0,
            AhciPortRegs::Tfd => Self::PORT_TFD,
            AhciPortRegs::Sig => Self::PORT_SIG,
            AhciPortRegs::Ssts => Self::PORT_SSTS,
            AhciPortRegs::Sctl => Self::PORT_SCTL,
            AhciPortRegs::Serr => Self::PORT_SERR,
            AhciPortRegs::Sact => Self::PORT_SACT,
            AhciPortRegs::Ci => Self::PORT_CI,
            AhciPortRegs::Sntf => Self::PORT_SNTF,
            AhciPortRegs::Fbs => Self::PORT_FBS,
        };

        port_offset + reg_offset
    }

    fn get_port_array_reg_offset(&self, port: u64, reg_enum: AhciPortArrayRegs, idx: u64) -> u64 {
        assert!(port < 32);

        let port_offset = self.base + Self::PORTS + 128 * port;

        let reg_offset = match reg_enum {
            AhciPortArrayRegs::Clb => {
                assert!(idx < 2);
                Self::PORT_CLB
            }
            AhciPortArrayRegs::Fb => {
                assert!(idx < 2);
                Self::PORT_FB
            }
            AhciPortArrayRegs::Rsv1 => {
                assert!(idx < 11);
                Self::PORT_RSV1
            }
            AhciPortArrayRegs::Vendor => {
                assert!(idx < 4);
                Self::PORT_VENDOR
            }
        };

        port_offset + reg_offset + 4 * idx
    }
}

impl AhciBarRegion for AhciBar {
    fn get_base(&self) -> u64 {
        self.base
    }

    fn read_reg(&self, reg_enum: AhciRegs) -> u32 {
        let offset: u64;
        match reg_enum {
            AhciRegs::Cap => offset = self.cap.offset,
            AhciRegs::Ghc => offset = self.ghc.offset,
            AhciRegs::Is => offset = self.is.offset,
            AhciRegs::Pi => offset = self.pi.offset,
            AhciRegs::Vs => offset = self.vs.offset,
            AhciRegs::Cccctl => offset = self.cccctl.offset,
            AhciRegs::Cccpts => offset = self.cccpts.offset,
            AhciRegs::Emloc => offset = self.emloc.offset,
            AhciRegs::Emctl => offset = self.emctl.offset,
            AhciRegs::Cap2 => offset = self.cap2.offset,
            AhciRegs::Bohc => offset = self.bohc.offset,
        }
        unsafe { ptr::read_volatile((self.base + offset) as *const u32) }
    }

    fn read_reg_idx(&self, reg_enum: AhciArrayRegs, idx: u64) -> u32 {
        let reg: ArrayRegister;
        match reg_enum {
            AhciArrayRegs::Rsv => reg = self.rsv,
            AhciArrayRegs::Vendor => reg = self.vendor,
        }

        if idx >= reg.num_regs {
            return 0;
        }
        unsafe { ptr::read_volatile((self.base + reg.offset + reg.multiplier * idx) as *const u32) }
    }

    fn write_reg(&self, reg_enum: AhciRegs, val: u32) {
        let offset: u64;
        match reg_enum {
            AhciRegs::Cap => offset = self.cap.offset,
            AhciRegs::Ghc => offset = self.ghc.offset,
            AhciRegs::Is => offset = self.is.offset,
            AhciRegs::Pi => offset = self.pi.offset,
            AhciRegs::Vs => offset = self.vs.offset,
            AhciRegs::Cccctl => offset = self.cccctl.offset,
            AhciRegs::Cccpts => offset = self.cccpts.offset,
            AhciRegs::Emloc => offset = self.emloc.offset,
            AhciRegs::Emctl => offset = self.emctl.offset,
            AhciRegs::Cap2 => offset = self.cap2.offset,
            AhciRegs::Bohc => offset = self.bohc.offset,
        }
        unsafe {
            ptr::write_volatile((self.base + offset) as *mut u32, val);
        }
    }

    fn write_reg_idx(&self, reg_enum: AhciArrayRegs, idx: u64, val: u32) {
        let reg: ArrayRegister;
        match reg_enum {
            AhciArrayRegs::Rsv => reg = self.rsv,
            AhciArrayRegs::Vendor => reg = self.vendor,
        }

        if idx < reg.num_regs {
            unsafe {
                ptr::write_volatile(
                    (self.base + reg.offset + reg.multiplier * idx) as *mut u32,
                    val,
                )
            }
        }
    }

    fn read_port_reg(&self, port: u64, reg_enum: AhciPortRegs) -> u32 {
        let offset = self.get_port_reg_offset(port, reg_enum);
        unsafe { ptr::read_volatile(offset as *const u32) }
    }

    fn write_port_reg(&self, port: u64, reg_enum: AhciPortRegs, val: u32) {
        let offset = self.get_port_reg_offset(port, reg_enum);
        unsafe {
            ptr::write_volatile(offset as *mut u32, val);
        }
    }

    fn read_port_reg_idx(&self, port: u64, reg_enum: AhciPortArrayRegs, idx: u64) -> u32 {
        let offset = self.get_port_array_reg_offset(port, reg_enum, idx);
        unsafe { ptr::read_volatile(offset as *const u32) }
    }

    fn write_port_reg_idx(&self, port: u64, reg_enum: AhciPortArrayRegs, idx: u64, val: u32) {
        let offset = self.get_port_array_reg_offset(port, reg_enum, idx);
        unsafe {
            ptr::write_volatile(offset as *mut u32, val);
        }
    }
}
