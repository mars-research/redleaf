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

    fn read_reg(&self, reg: AhciRegs) -> u64;
    fn write_reg(&self, reg: AhciRegs, val: u64);

    fn read_reg_idx(&self, reg: AhciArrayRegs, idx: u64) -> u64;
    fn write_reg_idx(&self, reg: AhciArrayRegs, idx: u64, val: u64);

    /*
    fn read_port_reg(&self, port: u8, reg: AhciPortRegs) -> u64;
    fn write_port_reg(&self, port: u8, reg: AhciPortRegs, val: u64);

    fn read_port_reg_idx(&self, port: u8, reg: AhciPortArrayRegs, idx: u64) -> u64;
    fn write_port_reg_idx(&self, port: u8, reg: AhciPortArrayRegs, idx: u64, val: u64);
    */
}
