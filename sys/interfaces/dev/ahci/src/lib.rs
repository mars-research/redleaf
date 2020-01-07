#![no_std]

#[derive(Copy, Clone)]
pub enum AhciRegs {
    Cap,
    Ghc,
    Is,
    Pi,
    Vs,
    Cccctl,
    Cccpts,
    Emloc,
    Emctl,
    Cap2,
    Bohc,
}

#[derive(Copy, Clone)]
pub enum AhciArrayRegs {
    Rsv,
    Vendor,
}

#[derive(Copy, Clone)]
pub enum AhciPortRegs {
    Is,
    Ie,
    Cmd,
    Rsv0,
    Tfd,
    Sig,
    Ssts,
    Sctl,
    Serr,
    Sact,
    Ci,
    Sntf,
    Fbs,
}

#[derive(Copy, Clone)]
pub enum AhciPortArrayRegs {
    Clb,
    Fb,
    Rsv1,
    Vendor,
}

pub trait AhciBarRegion {
    fn get_base(&self) -> u64;

    fn read_reg(&self, reg: AhciRegs) -> u32;
    fn write_reg(&self, reg: AhciRegs, val: u32);

    fn read_regf(&self, reg: AhciRegs, flags: u32) -> bool {
        (self.read_reg(reg) & flags) as u32 == flags
    }
    fn write_regf(&self, reg: AhciRegs, flags: u32, value: bool) {
        let orig = self.read_reg(reg);

        let tmp = match value {
            true => orig | flags,
            false => orig & !flags,
        };

        self.write_reg(reg, tmp);
    }

    fn read_reg_idx(&self, reg: AhciArrayRegs, idx: u64) -> u32;
    fn write_reg_idx(&self, reg: AhciArrayRegs, idx: u64, val: u32);

    fn read_port_reg(&self, port: u64, reg: AhciPortRegs) -> u32;
    fn write_port_reg(&self, port: u64, reg: AhciPortRegs, val: u32);

    fn read_port_reg_idx(&self, port: u64, reg: AhciPortArrayRegs, idx: u64) -> u32;
    fn write_port_reg_idx(&self, port: u64, reg: AhciPortArrayRegs, idx: u64, val: u32);

    fn read_port_regf(&self, port: u64, reg: AhciPortRegs, flags: u32) -> bool {
        (self.read_port_reg(port, reg) & flags) as u32 == flags
    }
    fn write_port_regf(&self, port: u64, reg: AhciPortRegs, flags: u32, value: bool) {
        let orig = self.read_port_reg(port, reg);

        let tmp = match value {
            true => orig | flags,
            false => orig & !flags,
        };

        self.write_port_reg(port, reg, tmp);
    }
}
