use alloc::{vec::Vec, collections::VecDeque};

#[derive(Copy, Clone, Debug)]
pub enum TpmRegs {
    TPM_ACCESS = 0x0000,
    TPM_INT_ENABLE = 0x0008,
    TPM_INT_VECTOR = 0x000C,
    TPM_INT_STATS = 0x0010,
    TPM_INTF_CAPABILITY = 0x0014,
    TPM_STS = 0x0018,
    TPM_DATA_FIFO = 0x0024,
    TPM_xDATA_FIFO = 0x0083,
    TPM_DID_VID = 0x0F00,
    TPM_RID = 0x0F04,
}

pub trait TpmDev: Send {
    fn read_reg(&self, locality: u32, reg: TpmRegs, buf: &mut Vec<u8>);

    fn write_reg(&self, locality: u32, reg: TpmRegs, buf: &Vec<u8>);
}
