use core::ptr;
use ixgbe::{IxgbeRegs, IxgbeArrayRegs, IxgbeBarRegion};
use syscalls::PciBar;
use alloc::boxed::Box;
use crate::interrupt::{disable_irq, enable_irq};

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
    dca_rxctrl: ArrayRegister,
    dtxmxszrq: Register,
    dmatxctl: Register,
    rttdcs: Register,
    tdbal: ArrayRegister,
    tdbah: ArrayRegister,
    tdlen: ArrayRegister,
    tdh: ArrayRegister,
    tdt: ArrayRegister,
    txdctl: ArrayRegister,
    dca_txctrl: ArrayRegister,
    txpbsize: ArrayRegister,
    txpbthresh: ArrayRegister,
    ral: ArrayRegister,
    rah: ArrayRegister,
    eicr: Register,
    eims: Register,
    eimc: Register,
    eiac: Register,
    gpie: Register,
    ivar: ArrayRegister,
    eitr: ArrayRegister,
    txdgpc: Register,
    txdgbch: Register,
    txdgbcl: Register,
    qptc: ArrayRegister,
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
    const SRRCTL: u64 = 0x02100;
    const RDRXCTL: u64 = 0x02F00;
    const RXPBSIZE: u64 = 0x03C00;
    const RXCTRL: u64 = 0x03000;
    const DCA_RXCTRL: u64 = 0x0100C;

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
    const TXPBTHRESH: u64 = 0x04950;
    const DCA_TXCTRL: u64 = 0x07200;

    const RAL: u64 = 0x0A200;
    const RAH: u64 = 0x0A204;

    const EICR: u64 = 0x00800;
    const EIMS: u64 = 0x00880;
    const EIMC: u64 = 0x00888;
    const EIAC: u64 = 0x00810;
    const EITR: u64 = 0x00820;
    const GPIE: u64 = 0x00898;

    const IVAR: u64 =0x00900;
    const TXDGPC: u64 = 0x087A0;
    const TXDGBCL: u64 = 0x087A4;
    const TXDGBCH: u64 = 0x087A8;
    const QPTC: u64 = 0x06030;

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
            srrctl: reg_ixgbe_mult!(SRRCTL, 16, 0x4),
            dca_rxctrl: reg_ixgbe_mult!(DCA_RXCTRL, 64, 0x40),

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
            dca_txctrl: reg_ixgbe_mult!(DCA_TXCTRL, 128, 0x40),
            txpbsize: reg_ixgbe_mult!(TXPBSIZE, 8, 0x4),
            ral: reg_ixgbe_mult!(RAL, 128, 0x8),
            rah: reg_ixgbe_mult!(RAH, 128, 0x8),

            eicr: reg_ixgbe!(EICR),
            eims: reg_ixgbe!(EIMS),
            eimc: reg_ixgbe!(EIMC),
            eiac: reg_ixgbe!(EIAC),
            gpie: reg_ixgbe!(GPIE),
            ivar: reg_ixgbe_mult!(IVAR, 64, 0x4),
            eitr: reg_ixgbe_mult!(EITR, 24, 0x4),
            txdgpc: reg_ixgbe!(TXDGPC),
            txdgbch: reg_ixgbe!(TXDGBCH),
            txdgbcl: reg_ixgbe!(TXDGBCL),
            qptc: reg_ixgbe_mult!(QPTC, 16, 0x40),
            txpbthresh: reg_ixgbe_mult!(TXPBTHRESH, 8, 0x4),
        }
    }

    #[inline(always)]
    fn get_offset(&self, reg_enum: IxgbeRegs) -> u64 {
         match reg_enum {
            IxgbeRegs::Ctrl => { self.ctrl.offset },
            IxgbeRegs::Status => { self.status.offset },
            IxgbeRegs::Ctrlext => { self.ctrl_ext.offset },
            IxgbeRegs::Eec => { self.eec.offset },
            IxgbeRegs::Autoc => { self.autoc.offset },
            IxgbeRegs::Gprc => { self.gprc.offset },
            IxgbeRegs::Gptc => { self.gptc.offset },
            IxgbeRegs::Gorcl => { self.gorcl.offset },
            IxgbeRegs::Gorch => { self.gorch.offset },
            IxgbeRegs::Gotcl => { self.gotcl.offset },
            IxgbeRegs::Gotch => { self.gotch.offset },
            IxgbeRegs::Hlreg0 => { self.hlreg0.offset },
            IxgbeRegs::Links => { self.links.offset },
            IxgbeRegs::Fctrl => { self.fctrl.offset },
            IxgbeRegs::Rdrxctl => { self.rdrxctl.offset },
            IxgbeRegs::Rxctrl => { self.rxctrl.offset },
            IxgbeRegs::Dtxmxszrq => { self.dtxmxszrq.offset },
            IxgbeRegs::Dmatxctl => { self.dmatxctl.offset },
            IxgbeRegs::Rttdcs => { self.rttdcs.offset },
            IxgbeRegs::Eicr => { self.eicr.offset },
            IxgbeRegs::Eims => { self.eims.offset },
            IxgbeRegs::Eimc => { self.eimc.offset },
            IxgbeRegs::Eiac => { self.eiac.offset },
            IxgbeRegs::Gpie => { self.gpie.offset },
            IxgbeRegs::Txdgpc => { self.txdgpc.offset },
            IxgbeRegs::Txdgbch => { self.txdgbch.offset },
            IxgbeRegs::Txdgbcl => { self.txdgbcl.offset },
         }
    }

    #[inline(always)]
    fn get_array_reg(&self, areg_enum: IxgbeArrayRegs) -> ArrayRegister {
        match areg_enum {
            IxgbeArrayRegs::Tdt => { self.tdt },
            IxgbeArrayRegs::Rdh => { self.rdh },
            IxgbeArrayRegs::Rdt => { self.rdt },
            IxgbeArrayRegs::Tdh => { self.tdh },
            IxgbeArrayRegs::Rdbal => { self.rdbal },
            IxgbeArrayRegs::Rdbah => { self.rdbah },
            IxgbeArrayRegs::Rdlen => { self.rdlen },
            IxgbeArrayRegs::Rxdctl => { self.rxdctl },
            IxgbeArrayRegs::DcaRxctrl => { self.dca_rxctrl },
            IxgbeArrayRegs::Srrctl => { self.srrctl },
            IxgbeArrayRegs::Rxpbsize => { self.rxpbsize },
            IxgbeArrayRegs::Tdbal => { self.tdbal },
            IxgbeArrayRegs::Tdbah => { self.tdbah },
            IxgbeArrayRegs::Tdlen => { self.tdlen },
            IxgbeArrayRegs::Txdctl => { self.txdctl },
            IxgbeArrayRegs::DcaTxctrl => { self.dca_txctrl },
            IxgbeArrayRegs::Txpbsize => { self.txpbsize },
            IxgbeArrayRegs::TxpbThresh => { self.txpbthresh },
            IxgbeArrayRegs::Ral => { self.ral },
            IxgbeArrayRegs::Rah => { self.rah },
            IxgbeArrayRegs::Ivar => { self.ivar },
            IxgbeArrayRegs::Eitr => { self.eitr },
            IxgbeArrayRegs::Qptc => { self.qptc },
        }
    }
}

impl IxgbeBarRegion for IxgbeBar {
    #[inline(always)]
    fn read_reg(&self, reg_enum: IxgbeRegs) -> u64 {
        disable_irq();
        let offset: u64 = self.get_offset(reg_enum);
        let ret = unsafe {
            ptr::read_volatile((self.base + offset) as *const u64) & 0xFFFF_FFFF as u64
        };
        enable_irq();
        ret
    }

    #[inline(always)]
    fn read_reg_idx(&self, reg_enum: IxgbeArrayRegs, idx: u64) -> u64 {
        disable_irq();
        let reg = self.get_array_reg(reg_enum);

        if idx >= reg.num_regs {
            return 0;
        }
        let ret = unsafe {
            ptr::read_volatile((self.base + reg.offset + reg.multiplier * idx) as *const u64) & 0xFFFF_FFFF as u64
        };
        enable_irq();
        ret
    }

    #[inline(always)]
    fn write_reg(&self, reg_enum: IxgbeRegs, val: u64) {
        disable_irq();
        let offset: u64 = self.get_offset(reg_enum);
        //println!("Write to {:08x}", self.base + offset);
        unsafe {
            ptr::write_volatile((self.base + offset) as *mut u32, val as u32);
        }
        enable_irq();
    }

    #[inline(always)]
    fn write_reg_tdt(&self, idx: u64, val: u64) {
        disable_irq();
        let reg = self.tdt;

        unsafe {
            ptr::write_volatile((self.base + reg.offset + reg.multiplier * idx) as *mut u32, val as u32)
        }
        enable_irq();
    }

    #[inline(always)]
    fn write_reg_idx(&self, reg_enum: IxgbeArrayRegs, idx: u64, val: u64) {
        disable_irq();
        let reg = self.get_array_reg(reg_enum);

        if idx < reg.num_regs {
            unsafe {
                ptr::write_volatile((self.base + reg.offset + reg.multiplier * idx) as *mut u32, val as u32)
            }
        }
        enable_irq();
    }
}
