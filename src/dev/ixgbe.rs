use core::ptr;
use ixgbe::{IxgbeRegs, IxgbeArrayRegs, IxgbeBarRegion};
use syscalls::PciBar;
use alloc::boxed::Box;

macro_rules! reg_ixgbe {
    ($off: ident) => {
        Register {
            offset: IxgbeBar::$off,
        }
    }
}

macro_rules! reg_ixgbe_mult {
    ($off: ident, $num: expr, $mult: expr) => {
        ArrayRegister {
            offset: IxgbeBar::$off,
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

pub struct IxgbeBar {
    base: u64,
    size: usize,
    ctrl: Register,
    status: Register,
    ctrl_ext: Register,
    eec: Register,
    autoc: Register,
    gprc: Register,
    gptc: Register,
    gorcl: Register,
    gorch: Register,
    gotcl: Register,
    gotch: Register,
    hlreg0: Register,
    links: Register,
    fctrl: Register,
    rdbal: ArrayRegister,
    rdbah: ArrayRegister,
    rdlen: ArrayRegister,
    rdh: ArrayRegister,
    rdt: ArrayRegister,
    rxdctl: ArrayRegister,
    srrctl: ArrayRegister,
    rdrxctl: Register,
    rxpbsize: ArrayRegister,
    rxctrl: Register,
    dtxmxszrq: Register,
    dmatxctl: Register,
    rttdcs: Register,
    tdbal: ArrayRegister,
    tdbah: ArrayRegister,
    tdlen: ArrayRegister,
    tdh: ArrayRegister,
    tdt: ArrayRegister,
    txdctl: ArrayRegister,
    txpbsize: ArrayRegister,
    ral: ArrayRegister,
    rah: ArrayRegister,
    eicr: Register,
    eims: Register,
    eimc: Register,
    eiac: Register,
}

impl IxgbeBar {
    const CTRL: u64 = 0x00000;
    const STATUS: u64 = 0x00004;
    const CTRL_EXT: u64 = 0x00018;
    const EEC: u64 = 0x10010;

    const AUTOC: u64 = 0x042A0;

    const GPRC: u64 = 0x04074;
    const GPTC: u64 = 0x04080;
    const GORCL: u64 = 0x04088;
    const GORCH: u64 = 0x0408C;
    const GOTCL: u64 = 0x04090;
    const GOTCH: u64 = 0x04094;

    const HLREG0: u64 = 0x04240;
    const LINKS: u64 = 0x042A4;

    const FCTRL: u64 = 0x05080;

    const RDBAL: u64 = 0x01000;
    const RDBAH: u64 = 0x01004;
    const RDLEN: u64 = 0x01008;
    const RDH: u64 = 0x01010;
    const RDT: u64 = 0x01018;
    const RXDCTL: u64 = 0x01028;
    const SRRCTL: u64 = 0x01014;
    const RDRXCTL: u64 = 0x02F00;
    const RXPBSIZE: u64 = 0x03C00;
    const RXCTRL: u64 = 0x03000;

    const DTXMXSZRQ: u64 = 0x08100;
    const DMATXCTL: u64 = 0x04A80;
    const RTTDCS: u64 = 0x04900;

    const TDBAL: u64 = 0x06000;
    const TDBAH: u64 = 0x06004;
    const TDLEN: u64 = 0x06008;
    const TDH: u64 = 0x06010;
    const TDT: u64 = 0x06018;
    const TXDCTL: u64 = 0x06028;
    const TXPBSIZE: u64 = 0x0CC00;

    const RAL: u64 = 0x0A200;
    const RAH: u64 = 0x0A204;

    const EICR: u64 = 0x00800;
    const EIMS: u64 = 0x00880;
    const EIMC: u64 = 0x00888;
    const EIAC: u64 = 0x00810;


    pub fn new(base: u64, size: usize) -> IxgbeBar {
        IxgbeBar {
            base,
            size,
            ctrl: reg_ixgbe!(CTRL),
            status: reg_ixgbe!(STATUS),
            ctrl_ext: reg_ixgbe!(CTRL_EXT),
            eec: reg_ixgbe!(EEC),
            autoc: reg_ixgbe!(AUTOC),
            gprc: reg_ixgbe!(GPRC),
            gptc: reg_ixgbe!(GPTC),
            gorcl: reg_ixgbe!(GORCL),
            gorch: reg_ixgbe!(GORCH),
            gotcl: reg_ixgbe!(GOTCL),
            gotch: reg_ixgbe!(GOTCH),
            hlreg0: reg_ixgbe!(HLREG0),
            links: reg_ixgbe!(LINKS),
            fctrl: reg_ixgbe!(FCTRL),

            rdbal: reg_ixgbe_mult!(RDBAL, 64, 0x40),
            rdbah: reg_ixgbe_mult!(RDBAH, 64, 0x40),
            rdlen: reg_ixgbe_mult!(RDLEN, 64, 0x60),
            rdh: reg_ixgbe_mult!(RDH, 64, 0x40),
            rdt: reg_ixgbe_mult!(RDT, 64, 0x40),
            rxdctl: reg_ixgbe_mult!(RXDCTL, 64, 0x40),
            srrctl: reg_ixgbe_mult!(SRRCTL, 64, 0x40),

            rdrxctl: reg_ixgbe!(RDRXCTL),
            rxpbsize: reg_ixgbe_mult!(RXPBSIZE, 8, 0x4),
            rxctrl: reg_ixgbe!(RXCTRL),
            dtxmxszrq: reg_ixgbe!(DTXMXSZRQ),
            dmatxctl: reg_ixgbe!(DMATXCTL),
            rttdcs: reg_ixgbe!(RTTDCS),

            tdbal: reg_ixgbe_mult!(TDBAL, 64, 0x40),
            tdbah: reg_ixgbe_mult!(TDBAH, 64, 0x40),
            tdlen: reg_ixgbe_mult!(TDLEN, 64, 0x40),
            tdh: reg_ixgbe_mult!(TDH, 64, 0x40),
            tdt: reg_ixgbe_mult!(TDT, 64, 0x40),
            txdctl: reg_ixgbe_mult!(TXDCTL, 64, 0x40),
            txpbsize: reg_ixgbe_mult!(TXPBSIZE, 8, 0x4),
            ral: reg_ixgbe_mult!(RAL, 128, 0x8),
            rah: reg_ixgbe_mult!(RAH, 128, 0x8),

            eicr: reg_ixgbe!(EICR),
            eims: reg_ixgbe!(EIMS),
            eimc: reg_ixgbe!(EIMC),
            eiac: reg_ixgbe!(EIAC),
        }
    }
}

impl IxgbeBarRegion for IxgbeBar {
    fn read_reg(&self, reg_enum: IxgbeRegs) -> u64 {
        let offset: u64;
        match reg_enum {
            IxgbeRegs::Ctrl => { offset = self.ctrl.offset },
            IxgbeRegs::Status => { offset = self.status.offset },
            IxgbeRegs::Ctrlext => { offset = self.ctrl_ext.offset },
            IxgbeRegs::Eec => { offset = self.eec.offset },
            IxgbeRegs::Autoc => { offset = self.autoc.offset },
            IxgbeRegs::Gprc => { offset = self.gprc.offset },
            IxgbeRegs::Gptc => { offset = self.gptc.offset },
            IxgbeRegs::Gorcl => { offset = self.gorcl.offset },
            IxgbeRegs::Gorch => { offset = self.gorch.offset },
            IxgbeRegs::Gotcl => { offset = self.gotcl.offset },
            IxgbeRegs::Gotch => { offset = self.gotch.offset },
            IxgbeRegs::Hlreg0 => { offset = self.hlreg0.offset },
            IxgbeRegs::Links => { offset = self.links.offset },
            IxgbeRegs::Fctrl => { offset = self.fctrl.offset },
            IxgbeRegs::Rdrxctl => { offset = self.rdrxctl.offset },
            IxgbeRegs::Rxctrl => { offset = self.rxctrl.offset },
            IxgbeRegs::Dtxmxszrq => { offset = self.dtxmxszrq.offset },
            IxgbeRegs::Dmatxctl => { offset = self.dmatxctl.offset },
            IxgbeRegs::Rttdcs => { offset = self.rttdcs.offset },
            IxgbeRegs::Eicr => { offset = self.eicr.offset },
            IxgbeRegs::Eims => { offset = self.eims.offset },
            IxgbeRegs::Eimc => { offset = self.eimc.offset },
            IxgbeRegs::Eiac => { offset = self.eiac.offset },
        }
        unsafe {
            ptr::read_volatile((self.base + offset) as *const u64)
        }
    }

    fn read_reg_idx(&self, reg_enum: IxgbeArrayRegs, idx: u64) -> u64 {
        let reg: ArrayRegister;
        match reg_enum {
            IxgbeArrayRegs::Rdbal => { reg = self.rdbal },
            IxgbeArrayRegs::Rdbah => { reg = self.rdbah },
            IxgbeArrayRegs::Rdlen => { reg = self.rdlen },
            IxgbeArrayRegs::Rdh => { reg = self.rdh },
            IxgbeArrayRegs::Rdt => { reg = self.rdt },
            IxgbeArrayRegs::Rxdctl => { reg = self.rxdctl },
            IxgbeArrayRegs::Srrctl => { reg = self.srrctl },
            IxgbeArrayRegs::Rxpbsize => { reg = self.rxpbsize },
            IxgbeArrayRegs::Tdbal => { reg = self.tdbal },
            IxgbeArrayRegs::Tdbah => { reg = self.tdbah },
            IxgbeArrayRegs::Tdlen => { reg = self.tdlen },
            IxgbeArrayRegs::Tdh => { reg = self.tdh },
            IxgbeArrayRegs::Tdt => { reg = self.tdt },
            IxgbeArrayRegs::Txdctl => { reg = self.txdctl },
            IxgbeArrayRegs::Txpbsize => { reg = self.txpbsize },
            IxgbeArrayRegs::Ral => { reg = self.ral },
            IxgbeArrayRegs::Rah => { reg = self.rah },
        }

        if idx >= reg.num_regs {
            return 0;
        }
        unsafe {
            ptr::read_volatile((self.base + reg.offset + reg.multiplier * idx) as *const u64)
        }
    }

    fn write_reg(&self, reg_enum: IxgbeRegs, val: u64) {
        let offset: u64;
        match reg_enum {
            IxgbeRegs::Ctrl => { offset = self.ctrl.offset },
            IxgbeRegs::Status => { offset = self.status.offset },
            IxgbeRegs::Ctrlext => { offset = self.ctrl_ext.offset },
            IxgbeRegs::Eec => { offset = self.eec.offset },
            IxgbeRegs::Autoc => { offset = self.autoc.offset },
            IxgbeRegs::Gprc => { offset = self.gprc.offset },
            IxgbeRegs::Gptc => { offset = self.gptc.offset },
            IxgbeRegs::Gorcl => { offset = self.gorcl.offset },
            IxgbeRegs::Gorch => { offset = self.gorch.offset },
            IxgbeRegs::Gotcl => { offset = self.gotcl.offset },
            IxgbeRegs::Gotch => { offset = self.gotch.offset },
            IxgbeRegs::Hlreg0 => { offset = self.hlreg0.offset },
            IxgbeRegs::Links => { offset = self.links.offset },
            IxgbeRegs::Fctrl => { offset = self.fctrl.offset },
            IxgbeRegs::Rdrxctl => { offset = self.rdrxctl.offset },
            IxgbeRegs::Rxctrl => { offset = self.rxctrl.offset },
            IxgbeRegs::Dtxmxszrq => { offset = self.dtxmxszrq.offset },
            IxgbeRegs::Dmatxctl => { offset = self.dmatxctl.offset },
            IxgbeRegs::Rttdcs => { offset = self.rttdcs.offset },
            IxgbeRegs::Eicr => { offset = self.eicr.offset },
            IxgbeRegs::Eims => { offset = self.eims.offset },
            IxgbeRegs::Eimc => { offset = self.eimc.offset },
            IxgbeRegs::Eiac => { offset = self.eiac.offset },
        }
        unsafe {
            ptr::write_volatile((self.base + offset) as *mut u64, val);
        }
    }

    fn write_reg_idx(&self, reg_enum: IxgbeArrayRegs, idx: u64, val: u64) {
        let reg: ArrayRegister;
        match reg_enum {
            IxgbeArrayRegs::Rdbal => { reg = self.rdbal },
            IxgbeArrayRegs::Rdbah => { reg = self.rdbah },
            IxgbeArrayRegs::Rdlen => { reg = self.rdlen },
            IxgbeArrayRegs::Rdh => { reg = self.rdh },
            IxgbeArrayRegs::Rdt => { reg = self.rdt },
            IxgbeArrayRegs::Rxdctl => { reg = self.rxdctl },
            IxgbeArrayRegs::Srrctl => { reg = self.srrctl },
            IxgbeArrayRegs::Rxpbsize => { reg = self.rxpbsize },
            IxgbeArrayRegs::Tdbal => { reg = self.tdbal },
            IxgbeArrayRegs::Tdbah => { reg = self.tdbah },
            IxgbeArrayRegs::Tdlen => { reg = self.tdlen },
            IxgbeArrayRegs::Tdh => { reg = self.tdh },
            IxgbeArrayRegs::Tdt => { reg = self.tdt },
            IxgbeArrayRegs::Txdctl => { reg = self.txdctl },
            IxgbeArrayRegs::Txpbsize => { reg = self.txpbsize },
            IxgbeArrayRegs::Ral => { reg = self.ral },
            IxgbeArrayRegs::Rah => { reg = self.rah },
        }

        if idx < reg.num_regs {
            unsafe {
                ptr::write_volatile((self.base + reg.offset + reg.multiplier * idx) as *mut u64, val)
            }
        }
    }
}
