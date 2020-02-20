#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::unreadable_literal)]

pub const IXGBE_CTRL_LNK_RST: u64               = 0x00000008; /* Link Reset. Resets everything. */
pub const IXGBE_CTRL_RST: u64                   = 0x04000000; /* Reset (SW) */
pub const IXGBE_CTRL_RST_MASK: u64              = IXGBE_CTRL_LNK_RST | IXGBE_CTRL_RST;
pub const IXGBE_CTRL_PCIE_MASTER_DISABLE: u64              = 1 << 2;

pub const IXGBE_STATUS_PCIE_MASTER_STATUS: u64  = 1 << 19;
pub const IXGBE_CTRL_EXT_DRV_LOAD: u64          = 1 << 28;

pub const IXGBE_EEC_ARD: u64                    = 0x00000200; /* EEPROM Auto Read Done */
pub const IXGBE_RDRXCTL_DMAIDONE: u64           = 0x00000008; /* DMA init cycle done */

pub const IXGBE_AUTOC_LMS_SHIFT: u64            = 13;
pub const IXGBE_AUTOC_LMS_MASK: u64             = 0x7 << IXGBE_AUTOC_LMS_SHIFT;
pub const IXGBE_AUTOC_LMS_10G_SERIAL: u64       = 0x3 << IXGBE_AUTOC_LMS_SHIFT;
pub const IXGBE_AUTOC_10G_PMA_PMD_MASK: u64     = 0x00000180;
pub const IXGBE_AUTOC_10G_PMA_PMD_SHIFT: u64    = 7;
pub const IXGBE_AUTOC_10G_XAUI: u64             = 0x0 << IXGBE_AUTOC_10G_PMA_PMD_SHIFT;
pub const IXGBE_AUTOC_AN_RESTART: u64           = 0x00001000;

pub const IXGBE_RXCTRL_RXEN: u64                = 0x00000001; /* Enable Receiver */

pub const IXGBE_RXPBSIZE_128KB: u64             = 0x00020000; /* 128KB Packet Buffer */

pub const IXGBE_HLREG0_RXCRCSTRP: u64           = 0x00000002; /* bit  1 */
pub const IXGBE_HLREG0_LPBK: u64           = 1 << 15;
pub const IXGBE_RDRXCTL_CRCSTRIP: u64           = 0x00000002; /* CRC Strip */

pub const IXGBE_FCTRL_BAM: u64                  = 0x00000400; /* Broadcast Accept Mode */

pub const IXGBE_SRRCTL_DESCTYPE_MASK: u64       = 0x0E000000;
pub const IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF: u64 = 0x02000000;
pub const IXGBE_SRRCTL_DROP_EN: u64             = 0x10000000;

pub const IXGBE_CTRL_EXT_NS_DIS: u64            = 0x00010000; /* No Snoop disable */

pub const IXGBE_HLREG0_TXCRCEN: u64             = 0x00000001; /* bit  0 */
pub const IXGBE_HLREG0_TXPADEN: u64             = 0x00000400; /* bit 10 */

pub const IXGBE_TXPBSIZE_40KB: u64              = 0x0000A000; /* 40KB Packet Buffer */
pub const IXGBE_RTTDCS_ARBDIS: u64              = 0x00000040; /* DCB arbiter disable */

pub const IXGBE_DMATXCTL_TE: u64                = 0x1; /* Transmit Enable */

pub const IXGBE_RXDCTL_ENABLE: u64              = 0x02000000; /* Ena specific Rx Queue */
pub const IXGBE_TXDCTL_ENABLE: u64              = 0x02000000; /* Ena specific Tx Queue */

pub const IXGBE_FCTRL_MPE: u64                  = 0x00000100; /* Multicast Promiscuous Ena*/
pub const IXGBE_FCTRL_UPE: u64                  = 0x00000200; /* Unicast Promiscuous Ena */

pub const IXGBE_LINKS_UP: u64                   = 0x40000000;
pub const IXGBE_LINKS_SPEED_82599: u64          = 0x30000000;
pub const IXGBE_LINKS_SPEED_100_82599: u64      = 0x10000000;
pub const IXGBE_LINKS_SPEED_1G_82599: u64       = 0x20000000;
pub const IXGBE_LINKS_SPEED_10G_82599: u64      = 0x30000000;

pub const IXGBE_RXD_STAT_DD: u64                = 0x01; /* Descriptor Done */
pub const IXGBE_RXD_STAT_EOP: u64               = 0x02; /* End of Packet */
pub const IXGBE_RXDADV_STAT_DD: u64             = IXGBE_RXD_STAT_DD; /* Done */
pub const IXGBE_RXDADV_STAT_EOP: u64            = IXGBE_RXD_STAT_EOP; /* End of Packet */

pub const IXGBE_ADVTXD_PAYLEN_SHIFT: u32        = 14; /* Adv desc PAYLEN shift */
pub const IXGBE_TXD_CMD_EOP: u32                = 0x01000000; /* End of Packet */
pub const IXGBE_ADVTXD_DCMD_EOP: u32            = IXGBE_TXD_CMD_EOP; /* End of Packet */
pub const IXGBE_TXD_CMD_RS: u32                 = 0x08000000; /* Report Status */
pub const IXGBE_ADVTXD_DCMD_RS: u32             = IXGBE_TXD_CMD_RS; /* Report Status */
pub const IXGBE_TXD_CMD_IFCS: u32               = 0x02000000; /* Insert FCS (Ethernet CRC) */
pub const IXGBE_ADVTXD_DCMD_IFCS: u32           = IXGBE_TXD_CMD_IFCS; /* Insert FCS */
pub const IXGBE_TXD_CMD_DEXT: u32               = 0x20000000; /* Desc extension (0 = legacy) */
pub const IXGBE_ADVTXD_DTYP_DATA: u32           = 0x00300000; /* Adv Data Descriptor */
pub const IXGBE_ADVTXD_DCMD_DEXT: u32           = IXGBE_TXD_CMD_DEXT; /* Desc ext 1=Adv */
pub const IXGBE_TXD_STAT_DD: u32                = 0x00000001; /* Descriptor Done */
pub const IXGBE_ADVTXD_STAT_DD: u32             = IXGBE_TXD_STAT_DD; /* Descriptor Done */

pub const IXGBE_IVAR_ALLOC_VAL: u32             = 0x80; /* Interrupt Allocation valid */
pub const IXGBE_EICR_RTX_QUEUE: u64             = 0x0000FFFF; /* RTx Queue Interrupt */

/* Interrupt clear mask */
pub const IXGBE_IRQ_CLEAR_MASK: u64                                    = 0xFFFFFFFF;

pub const IXGBE_GPIE_MSIX_MODE: u64                                    = 0x00000010; /* MSI-X mode */
pub const IXGBE_GPIE_OCD: u64                                          = 0x00000020; /* Other Clear Disable */
pub const IXGBE_GPIE_EIMEN: u64                                        = 0x00000040; /* Immediate Interrupt Enable */
pub const IXGBE_GPIE_EIAME: u64                                        = 0x40000000;
pub const IXGBE_GPIE_PBA_SUPPORT: u64                                  = 0x80000000;

/*
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_rx_desc_read {
    pub pkt_addr: u64,
    /* Packet buffer address */
    pub hdr_addr: u64,
    /* Header buffer address */
}

/* Receive Descriptor - Advanced */
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_rx_desc_wb_lower_lo_dword_hs_rss {
    pub pkt_info: u16,
    /* RSS, Pkt type */
    pub hdr_info: u16,
    /* Splithdr, hdrlen */
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub union ixgbe_adv_rx_desc_wb_lower_lo_dword {
    pub data: u32,
    pub hs_rss: ixgbe_adv_rx_desc_wb_lower_lo_dword_hs_rss,
}

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_rx_desc_wb_lower_hi_dword_csum_ip {
    pub ip_id: u16,
    /* IP id */
    pub csum: u16,
    /* Packet Checksum */
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub union ixgbe_adv_rx_desc_wb_lower_hi_dword {
    pub rss: u32,
    /* RSS Hash */
    pub csum_ip: ixgbe_adv_rx_desc_wb_lower_hi_dword_csum_ip,
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_rx_desc_wb_lower {
    pub lo_dword: ixgbe_adv_rx_desc_wb_lower_lo_dword,
    pub hi_dword: ixgbe_adv_rx_desc_wb_lower_hi_dword,
}

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_rx_desc_wb_upper {
    pub status_error: u32,
    /* ext status/error */
    pub length: u16,
    /* Packet length */
    pub vlan: u16,
    /* VLAN tag */
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_rx_desc_wb {
    pub lower: ixgbe_adv_rx_desc_wb_lower,
    pub upper: ixgbe_adv_rx_desc_wb_upper,
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub union ixgbe_adv_rx_desc {
    pub read: ixgbe_adv_rx_desc_read,
    pub wb: ixgbe_adv_rx_desc_wb, /* writeback */
    _union_align: [u64; 2],
}

/* Transmit Descriptor - Advanced */
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_tx_desc_read {
    pub buffer_addr: u64,
    /* Address of descriptor's data buf */
    pub cmd_type_len: u32,
    pub olinfo_status: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct ixgbe_adv_tx_desc_wb {
    pub rsvd: u64,
    /* Reserved */
    pub nxtseq_seed: u32,
    pub status: u32,
}

#[derive(Copy, Clone)]
#[repr(packed)]
pub union ixgbe_adv_tx_desc {
    pub read: ixgbe_adv_tx_desc_read,
    pub wb: ixgbe_adv_tx_desc_wb,
    _union_align: [u64; 2],
}*/
