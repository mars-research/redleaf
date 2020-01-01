#![no_std]

#[derive(Copy, Clone)]
pub enum IxgbeRegs {
    Ctrl,
    Status,
    Ctrlext,
    Eec,
    Autoc,
    Gprc,
    Gptc,
    Gorcl,
    Gorch,
    Gotcl,
    Gotch,
    Hlreg0,
    Links,
    Fctrl,
    Rdrxctl,
    Rxctrl,
    Dtxmxszrq,
    Dmatxctl,
    Rttdcs,
    Eicr,
    Eims,
    Eimc,
    Eiac,
}

#[derive(Copy, Clone)]
pub enum IxgbeArrayRegs {
    Rdbal,
    Rdbah,
    Rdlen,
    Rdh,
    Rdt,
    Rxdctl,
    DcaRxctrl,
    Srrctl,
    Rxpbsize,
    Tdbal,
    Tdbah,
    Tdlen,
    Tdh,
    Tdt,
    Txdctl,
    Txpbsize,
    Ral,
    Rah,
}

pub trait IxgbeBarRegion {
    fn read_reg(&self, reg: IxgbeRegs) -> u64;
    fn write_reg(&self, reg: IxgbeRegs, val: u64);

    fn read_reg_idx(&self, reg: IxgbeArrayRegs, idx: u64) -> u64;
    fn write_reg_idx(&self, reg: IxgbeArrayRegs, idx: u64, val: u64);
}
