#![no_std]

#[allow(non_camel_case_types)]
#[repr(usize)]
#[derive(Copy, Clone, Debug)]
pub enum IxgbeRegs {
    CTRL = 0x00000,
    STATUS = 0x00004,
    CTRLEXT = 0x00018,
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
    RDRXCTL = 0x02F00,
    RXCTRL = 0x03000,
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
}

#[allow(non_camel_case_types)]
#[repr(usize)]
#[derive(Copy, Clone, Debug)]
pub enum IxgbeArrayRegs {
    RDBAL = 0x01000,
    RDBAH = 0x01004,
    RDLEN = 0x01008,
    RDH = 0x01010,
    RDT = 0x01018,
    RXDCTL = 0x01028,
    SRRCTL = 0x01014,
    RXPBSIZE = 0x03C00,
    DCA_RXCTRL = 0x0100C,
    TDBAL = 0x06000,
    TDBAH = 0x06004,
    TDLEN = 0x06008,
    TDH = 0x06010,
    TDT = 0x06018,
    TXDCTL = 0x06028,
    TXPBSIZE = 0x0CC00,
    TXPBTHRESH = 0x04950,
    DCA_TXCTRL = 0x07200,
    RAL = 0x0A200,
    RAH = 0x0A204,
    EITR = 0x00820,
    IVAR = 0x00900,
    QPTC = 0x06030,
}

pub trait BarRegion {
    fn read_reg32(&self, offset: usize) -> u32;
    fn write_reg32(&self, offset: usize, val: u32);

    fn read_reg64(&self, offset: usize) -> u64;
    fn write_reg64(&self, offset: usize, val: u64);
}
