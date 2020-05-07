use core::ptr;
use platform::PciBarAddr;

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub enum NvmeRegs32 {
    VS = 0x8,
    INTMS = 0xC,
    INTMC = 0x10,
    CC = 0x14,
    CSTS = 0x1C,
    NSSR = 0x20,
    AQA = 0x24,
    CMBLOC = 0x38,
    CMBSZ = 0x3C,
    BPINFO = 0x40,
    BPRSEL = 0x44,
    BPMBL = 0x48,
    CMBSTS = 0x58,
    PMRCAP = 0xE00,
    PMRCTL = 0xE04,
    PMRSTS = 0xE08,
    PMREBS = 0xE0C,
    PMRSWTP = 0xE10,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub enum NvmeRegs64 {
    CAP = 0x0,
    ASQ = 0x28,
    ACQ = 0x30,
    CMBMSC = 0x50,
    PMRMSC = 0xE14,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
pub (crate) enum NvmeArrayRegs {
    SQyTDBL,
    CQyHDBL,
}
