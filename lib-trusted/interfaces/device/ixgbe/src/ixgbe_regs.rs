#![no_std]

use core::ptr;
use platform::PciBarAddr;

macro_rules! ixgbe_dmareg_mult {
    ($off: ident, $num: expr, $mult: expr) => {
        ArrayRegister {
            offset: IxgbeDmaRegs::$off,
            num_regs: $num,
            multiplier: $mult
        }
    }
}

macro_rules! ixgbe_nodma_reg_mult {
    ($off: ident, $num: expr, $mult: expr) => {
        ArrayRegister {
            offset: IxgbeNonDmaRegs::$off,
            num_regs: $num,
            multiplier: $mult
        }
    }
}


#[derive(Copy, Clone)]
struct ArrayRegister {
    offset: u64,
    num_regs: u64,
    multiplier: u64,
}


#[derive(Copy, Clone, Debug)]
pub enum IxgbeRegs {
    CTRL = 0x00000,
    STATUS = 0x00004,
    CTRL_EXT = 0x00018,
    EEC = 0x10010,
    AUTOC = 0x042A0,
    GPRC = 0x04074,
    GPTC = 0x04080,
    GORCL = 0x04088,
    GORCH = 0x0408C,
    GOTCL = 0x04090,
    GOTCH = 0x04094,
    HLREG0 = 0x04240,
    LINKS = 0x042A4,
    FCTRL = 0x05080,
    RXCTRL = 0x03000,
    RDRXCTL = 0x02F00,
    DTXMXSZRQ = 0x08100,
    DMATXCTL = 0x04A80,
    RTTDCS = 0x04900,
    EICR = 0x00800,
    EIMS = 0x00880,
    EIMC = 0x00888,
    EIAC = 0x00810,
    GPIE = 0x00898,
    TXDGPC = 0x087A0,
    TXDGBCL = 0x087A4,
    TXDGBCH = 0x087A8,
    CRCERRS = 0x04000,
    ILLERRC = 0x04004,
    ERRBC = 0x04008,
    MLFC = 0x04034,
    MRFC = 0x04038,
    RLEC = 0x04040,
    LXONRXCNT = 0x041A4,
    LXOFFRXCNT = 0x041A8,
    RXDGPC = 0x02F50,
    RXDGBCL = 0x02F54,
    RXDGBCH = 0x02F58,
    BPRC = 0x04078,
    MPRC = 0x0407c,
    BPTC = 0x040F4,
    MPTC = 0x040F0,
    RUC = 0x040A4,
    RFC = 0x040A8,
    ROC = 0x040AC,
    RJC = 0x040B0,
}

#[derive(Copy, Clone, Debug)]
pub enum IxgbeDmaArrayRegs {
    Rdbal,
    Rdbah,
    Rdlen,
    Rdh,
    Rdt,
    DcaRxctrl,
    Srrctl,
    Rxpbsize,
    Tdbal,
    Tdbah,
    Tdlen,
    Tdh,
    Tdt,
    DcaTxctrl,
    Txpbsize,
    TxpbThresh,
    Ivar,
    Eitr,
    Txdctl,
    Rxdctl,
}

pub struct IxgbeDmaRegs {
    bar: PciBarAddr,
    rdbal: ArrayRegister,
    rdbah: ArrayRegister,
    rdlen: ArrayRegister,
    rdh: ArrayRegister,
    rdt: ArrayRegister,
    srrctl: ArrayRegister,
    rxpbsize: ArrayRegister,
    dca_rxctrl: ArrayRegister,
    rxdctl: ArrayRegister,

    tdbal: ArrayRegister,
    tdbah: ArrayRegister,
    tdlen: ArrayRegister,
    tdh: ArrayRegister,
    tdt: ArrayRegister,
    txpbsize: ArrayRegister,
    txpbthresh: ArrayRegister,
    dca_txctrl: ArrayRegister,
    txdctl: ArrayRegister,

    ivar: ArrayRegister,
    eitr: ArrayRegister,
}


impl IxgbeDmaRegs {
    const RDBAL: u64 = 0x01000;
    const RDBAH: u64 = 0x01004;
    const RDLEN: u64 = 0x01008;
    const RDH: u64 = 0x01010;
    const RDT: u64 = 0x01018;
    const SRRCTL: u64 = 0x02100;
    const RXPBSIZE: u64 = 0x03C00;
    const DCA_RXCTRL: u64 = 0x0100C;
    const RXDCTL: u64 = 0x01028;

    const TDBAL: u64 = 0x06000;
    const TDBAH: u64 = 0x06004;
    const TDLEN: u64 = 0x06008;
    const TDH: u64 = 0x06010;
    const TDT: u64 = 0x06018;
    const TXPBSIZE: u64 = 0x0CC00;
    const TXPBTHRESH: u64 = 0x04950;
    const DCA_TXCTRL: u64 = 0x07200;
    const TXDCTL: u64 = 0x06028;

    const IVAR: u64 =0x00900;
    const EITR: u64 = 0x00820;

    pub unsafe fn new(bar: PciBarAddr) -> Self {
        IxgbeDmaRegs {
            bar,
            rdbal: ixgbe_dmareg_mult!(RDBAL, 64, 0x40),
            rdbah: ixgbe_dmareg_mult!(RDBAH, 64, 0x40),
            rdlen: ixgbe_dmareg_mult!(RDLEN, 64, 0x60),
            rdh: ixgbe_dmareg_mult!(RDH, 64, 0x40),
            rdt: ixgbe_dmareg_mult!(RDT, 64, 0x40),
            srrctl: ixgbe_dmareg_mult!(SRRCTL, 16, 0x4),
            dca_rxctrl: ixgbe_dmareg_mult!(DCA_RXCTRL, 64, 0x40),
            rxpbsize: ixgbe_dmareg_mult!(RXPBSIZE, 8, 0x4),
            rxdctl: ixgbe_dmareg_mult!(RXDCTL, 64, 0x40),

            tdbal: ixgbe_dmareg_mult!(TDBAL, 64, 0x40),
            tdbah: ixgbe_dmareg_mult!(TDBAH, 64, 0x40),
            tdlen: ixgbe_dmareg_mult!(TDLEN, 64, 0x40),
            tdh: ixgbe_dmareg_mult!(TDH, 64, 0x40),
            tdt: ixgbe_dmareg_mult!(TDT, 64, 0x40),
            txpbsize: ixgbe_dmareg_mult!(TXPBSIZE, 8, 0x4),
            txpbthresh: ixgbe_dmareg_mult!(TXPBTHRESH, 8, 0x4),
            dca_txctrl: ixgbe_dmareg_mult!(DCA_TXCTRL, 128, 0x40),
            txdctl: ixgbe_dmareg_mult!(TXDCTL, 64, 0x40),

            ivar: ixgbe_dmareg_mult!(IVAR, 64, 0x4),
            eitr: ixgbe_dmareg_mult!(EITR, 24, 0x4),
        }
    }

    #[inline(always)]
    fn get_array_reg(&self, areg_enum: IxgbeDmaArrayRegs) -> ArrayRegister {
        match areg_enum {
            IxgbeDmaArrayRegs::Tdt => { self.tdt },
            IxgbeDmaArrayRegs::Rdh => { self.rdh },
            IxgbeDmaArrayRegs::Rdt => { self.rdt },
            IxgbeDmaArrayRegs::Tdh => { self.tdh },
            IxgbeDmaArrayRegs::Rdbal => { self.rdbal },
            IxgbeDmaArrayRegs::Rdbah => { self.rdbah },
            IxgbeDmaArrayRegs::Rdlen => { self.rdlen },
            IxgbeDmaArrayRegs::DcaRxctrl => { self.dca_rxctrl },
            IxgbeDmaArrayRegs::Srrctl => { self.srrctl },
            IxgbeDmaArrayRegs::Rxpbsize => { self.rxpbsize },
            IxgbeDmaArrayRegs::Tdbal => { self.tdbal },
            IxgbeDmaArrayRegs::Tdbah => { self.tdbah },
            IxgbeDmaArrayRegs::Tdlen => { self.tdlen },
            IxgbeDmaArrayRegs::DcaTxctrl => { self.dca_txctrl },
            IxgbeDmaArrayRegs::Txpbsize => { self.txpbsize },
            IxgbeDmaArrayRegs::TxpbThresh => { self.txpbthresh },
            IxgbeDmaArrayRegs::Ivar => { self.ivar },
            IxgbeDmaArrayRegs::Eitr => { self.eitr },
            IxgbeDmaArrayRegs::Txdctl => { self.txdctl },
            IxgbeDmaArrayRegs::Rxdctl => { self.rxdctl },
        }
    }

    #[inline(always)]
    pub fn read_reg_idx(&self, reg_enum: IxgbeDmaArrayRegs, idx: u64) -> u64 {
        let reg = self.get_array_reg(reg_enum);

        if idx >= reg.num_regs {
            return 0;
        }
        let ret = unsafe {
            ptr::read_volatile((self.bar.get_base() as u64 + reg.offset + reg.multiplier * idx) as *const u64) & 0xFFFF_FFFF as u64
        };
        ret
    }


    #[inline(always)]
    pub fn write_reg_idx(&self, reg_enum: IxgbeDmaArrayRegs, idx: u64, val: u64) {
        let reg = self.get_array_reg(reg_enum);

        if idx < reg.num_regs {
            unsafe {
                ptr::write_volatile((self.bar.get_base() as u64 + reg.offset + reg.multiplier * idx) as *mut u32, val as u32)
            }
        }

    }
}

pub enum IxgbeNoDmaArrayRegs {
    Qptc,
    Rxmpc,
    Ral,
    Rah,
}

pub struct IxgbeNonDmaRegs {
    bar: PciBarAddr,
    qptc: ArrayRegister,
    rxmpc: ArrayRegister,
    ral: ArrayRegister,
    rah: ArrayRegister,
}

impl IxgbeNonDmaRegs {
    const QPTC: u64 = 0x06030;
    const RXMPC: u64 = 0x03FA0;

    const RAL: u64 = 0x0A200;
    const RAH: u64 = 0x0A204;

    pub unsafe fn new(bar: PciBarAddr) -> Self {
        IxgbeNonDmaRegs {
            bar,
            qptc: ixgbe_nodma_reg_mult!(QPTC, 16, 0x40),
            rxmpc: ixgbe_nodma_reg_mult!(RXMPC, 8, 0x4),
            ral: ixgbe_nodma_reg_mult!(RAL, 128, 0x8),
            rah: ixgbe_nodma_reg_mult!(RAH, 128, 0x8),
        }
    }

    #[inline(always)]
    fn get_array_reg(&self, areg_enum: IxgbeNoDmaArrayRegs) -> ArrayRegister {
        match areg_enum {
            IxgbeNoDmaArrayRegs::Qptc => { self.qptc },
            IxgbeNoDmaArrayRegs::Rxmpc => { self.rxmpc },
            IxgbeNoDmaArrayRegs::Ral => { self.ral },
            IxgbeNoDmaArrayRegs::Rah => { self.rah },
        }
    }
    #[inline(always)]
    pub fn read_reg_idx(&self, reg_enum: IxgbeNoDmaArrayRegs, idx: u64) -> u64 {
        let reg = self.get_array_reg(reg_enum);

        if idx >= reg.num_regs {
            return 0;
        }
        let ret = unsafe {
            ptr::read_volatile((self.bar.get_base() as u64 + reg.offset + reg.multiplier * idx) as *const u64) & 0xFFFF_FFFF as u64
        };
        ret
    }


    #[inline(always)]
    pub fn write_reg_idx(&self, reg_enum: IxgbeNoDmaArrayRegs, idx: u64, val: u64) {
        let reg = self.get_array_reg(reg_enum);

        if idx < reg.num_regs {
            unsafe {
                ptr::write_volatile((self.bar.get_base() as u64 + reg.offset + reg.multiplier * idx) as *mut u32, val as u32)
            }
        }

    }
}
