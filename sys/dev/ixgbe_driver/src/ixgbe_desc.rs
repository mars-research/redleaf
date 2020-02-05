#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::unreadable_literal)]

pub const IXGBE_CTRL_LNK_RST: u32               = 0x00000008; /* Link Reset. Resets everything. */
pub const IXGBE_CTRL_RST: u32                   = 0x04000000; /* Reset (SW) */
pub const IXGBE_CTRL_RST_MASK: u32              = (IXGBE_CTRL_LNK_RST | IXGBE_CTRL_RST);
pub const IXGBE_CTRL_PCIE_MASTER_DISABLE: u32              = 1 << 2;

pub const IXGBE_STATUS_PCIE_MASTER_STATUS: u32  = 1 << 19;
pub const IXGBE_CTRL_EXT_DRV_LOAD: u32          = 1 << 28;

pub const IXGBE_EEC_ARD: u32                    = 0x00000200; /* EEPROM Auto Read Done */
pub const IXGBE_RDRXCTL_DMAIDONE: u32           = 0x00000008; /* DMA init cycle done */

pub const IXGBE_AUTOC_LMS_SHIFT: u32            = 13;
pub const IXGBE_AUTOC_LMS_MASK: u32             = (0x7 << IXGBE_AUTOC_LMS_SHIFT);
pub const IXGBE_AUTOC_LMS_10G_SERIAL: u32       = (0x3 << IXGBE_AUTOC_LMS_SHIFT);
pub const IXGBE_AUTOC_10G_PMA_PMD_MASK: u32     = 0x00000180;
pub const IXGBE_AUTOC_10G_PMA_PMD_SHIFT: u32    = 7;
pub const IXGBE_AUTOC_10G_XAUI: u32             = (0x0 << IXGBE_AUTOC_10G_PMA_PMD_SHIFT);
pub const IXGBE_AUTOC_AN_RESTART: u32           = 0x00001000;

pub const IXGBE_RXCTRL_RXEN: u32                = 0x00000001; /* Enable Receiver */

pub const IXGBE_RXPBSIZE_128KB: u32             = 0x00020000; /* 128KB Packet Buffer */

pub const IXGBE_HLREG0_RXCRCSTRP: u32           = 0x00000002; /* bit  1 */
pub const IXGBE_HLREG0_LPBK: u32           = 1 << 15;
pub const IXGBE_RDRXCTL_CRCSTRIP: u32           = 0x00000002; /* CRC Strip */

pub const IXGBE_FCTRL_BAM: u32                  = 0x00000400; /* Broadcast Accept Mode */

pub const IXGBE_SRRCTL_DESCTYPE_MASK: u32       = 0x0E000000;
pub const IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF: u32 = 0x02000000;
pub const IXGBE_SRRCTL_DROP_EN: u32             = 0x10000000;

pub const IXGBE_CTRL_EXT_NS_DIS: u32            = 0x00010000; /* No Snoop disable */

pub const IXGBE_HLREG0_TXCRCEN: u32             = 0x00000001; /* bit  0 */
pub const IXGBE_HLREG0_TXPADEN: u32             = 0x00000400; /* bit 10 */

pub const IXGBE_TXPBSIZE_40KB: u32              = 0x0000A000; /* 40KB Packet Buffer */
pub const IXGBE_RTTDCS_ARBDIS: u32              = 0x00000040; /* DCB arbiter disable */

pub const IXGBE_DMATXCTL_TE: u32                = 0x1; /* Transmit Enable */

pub const IXGBE_RXDCTL_ENABLE: u32              = 0x02000000; /* Ena specific Rx Queue */
pub const IXGBE_TXDCTL_ENABLE: u32              = 0x02000000; /* Ena specific Tx Queue */

pub const IXGBE_FCTRL_MPE: u32                  = 0x00000100; /* Multicast Promiscuous Ena*/
pub const IXGBE_FCTRL_UPE: u32                  = 0x00000200; /* Unicast Promiscuous Ena */

pub const IXGBE_LINKS_UP: u32                   = 0x40000000;
pub const IXGBE_LINKS_SPEED_82599: u32          = 0x30000000;
pub const IXGBE_LINKS_SPEED_100_82599: u32      = 0x10000000;
pub const IXGBE_LINKS_SPEED_1G_82599: u32       = 0x20000000;
pub const IXGBE_LINKS_SPEED_10G_82599: u32      = 0x30000000;

pub const IXGBE_RXD_STAT_DD: u32                = 0x01; /* Descriptor Done */
pub const IXGBE_RXD_STAT_EOP: u32               = 0x02; /* End of Packet */
pub const IXGBE_RXDADV_STAT_DD: u32             = IXGBE_RXD_STAT_DD; /* Done */
pub const IXGBE_RXDADV_STAT_EOP: u32            = IXGBE_RXD_STAT_EOP; /* End of Packet */

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
pub const IXGBE_EICR_RTX_QUEUE: u32             = 0x0000FFFF; /* RTx Queue Interrupt */

/* Interrupt clear mask */
pub const IXGBE_IRQ_CLEAR_MASK: u32                                    = 0xFFFFFFFF;

pub const IXGBE_GPIE_MSIX_MODE: u32                                    = 0x00000010; /* MSI-X mode */
pub const IXGBE_GPIE_OCD: u32                                          = 0x00000020; /* Other Clear Disable */
pub const IXGBE_GPIE_EIMEN: u32                                        = 0x00000040; /* Immediate Interrupt Enable */
pub const IXGBE_GPIE_EIAME: u32                                        = 0x40000000;
pub const IXGBE_GPIE_PBA_SUPPORT: u32                                  = 0x80000000;
