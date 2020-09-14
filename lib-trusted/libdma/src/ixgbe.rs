// Datastructures from ixy.rs
use super::{Dma, DmaAllocator};
use super::zeroed_allocator;
use libsyscalls::errors::Result;

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
}

zeroed_allocator!([ixgbe_adv_tx_desc; 512]); // tx_desc
zeroed_allocator!([ixgbe_adv_rx_desc; 512]); // rx desc

zeroed_allocator!([ixgbe_adv_tx_desc; 32]); // tx_desc
zeroed_allocator!([ixgbe_adv_rx_desc; 32]); // rx desc

zeroed_allocator!([ixgbe_adv_rx_desc; 64]); // rx desc
zeroed_allocator!([u8; 2048]); // rx buffer

pub fn allocate_dma<T>() -> Result<Dma<T>>
    where T: DmaAllocator
{
    T::allocate()
}
