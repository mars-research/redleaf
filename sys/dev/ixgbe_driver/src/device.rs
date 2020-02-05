#![no_std]

use alloc::boxed::Box;
use alloc::vec::Vec;
use ixgbe::BarRegion;
use libdma::Dma;
use libdma::ahci::allocate_dma;
use crate::ixgbe_desc::*;
use libdma::ixgbe::{ixgbe_adv_rx_desc, ixgbe_adv_tx_desc};
use crate::Result;
use ixgbe::{IxgbeRegs, IxgbeArrayRegs};
use console::{println, print};
use core::{mem, slice, cmp};
use libtime::sys_ns_loopsleep;
use alloc::format;
use protocol::UdpPacket;
use byteorder::{ByteOrder, BigEndian, LittleEndian};

const ONE_MS_IN_NS: u64 = 100_0000;
const TX_CLEAN_BATCH: usize = 32;
const PACKET_SIZE: usize = 60;

pub struct Intel8259x {
    receive_buffer: [Dma<[u8; 2048]>; 32],
    receive_ring: Dma<[ixgbe_adv_rx_desc; 32]>,
    receive_index: usize,
    //transmit_buffer: [Dma<[u8; 2048]>; 512],
    transmit_ring: Dma<[ixgbe_adv_tx_desc; 512]>,
    transmit_ring_free: usize,
    transmit_index: usize,
    transmit_clean_index: usize,
    next_id: usize,
    pub bar: Box<dyn BarRegion>,
    counter: usize,
    gcounter: usize,
}

fn wrap_ring(index: usize, ring_size: usize) -> usize {
    (index + 1) & (ring_size - 1)
}

const fn IXGBE_RAL(i: usize) -> usize {
    IxgbeArrayRegs::RAL as usize + (i * 8)
}
const fn IXGBE_RAH(i: usize) -> usize {
    IxgbeArrayRegs::RAH as usize + (i * 8)
}
const fn IXGBE_RDBAL(i: usize) -> usize {
    IxgbeArrayRegs::RDBAL as usize + (i * 0x40)
}
const fn IXGBE_RDBAH(i: usize) -> usize {
    IxgbeArrayRegs::RDBAH as usize + (i * 0x40)
}
const fn IXGBE_RDLEN(i: usize) -> usize {
    IxgbeArrayRegs::RDLEN as usize + (i * 0x40)
}
const fn IXGBE_RDH(i: usize) -> usize {
    IxgbeArrayRegs::RDH as usize + (i * 0x40)
}
const fn IXGBE_RDT(i: usize) -> usize {
    IxgbeArrayRegs::RDT as usize + (i * 0x40)
}
const fn IXGBE_TDBAL(i: usize) -> usize {
    IxgbeArrayRegs::TDBAL as usize + (i * 0x40)
}
const fn IXGBE_TDBAH(i: usize) -> usize {
    IxgbeArrayRegs::TDBAH as usize + (i * 0x40)
}
const fn IXGBE_TDLEN(i: usize) -> usize {
    IxgbeArrayRegs::TDLEN as usize + (i * 0x40)
}
const fn IXGBE_TXDCTL(i: usize) -> usize {
    IxgbeArrayRegs::TXDCTL as usize + (i * 0x40)
}
const fn IXGBE_RXPBSIZE(i: usize) -> usize {
    IxgbeArrayRegs::RXPBSIZE as usize + (i * 4)
}
const fn IXGBE_SRRCTL(i: usize) -> usize {
    IxgbeArrayRegs::SRRCTL as usize + (i * 0x40)
}
const fn IXGBE_TXPBSIZE(i: usize) -> usize {
    IxgbeArrayRegs::TXPBSIZE as usize + (i * 4)
}
const fn IXGBE_TDH(i: usize) -> usize {
    IxgbeArrayRegs::TDH as usize + (i * 4)
}
pub const fn IXGBE_TDT(i: usize) -> usize {
    IxgbeArrayRegs::TDT as usize + (i * 4)
}
const fn IXGBE_IVAR(i: usize) -> usize {
    IxgbeArrayRegs::IVAR as usize + (i * 4)
}
const fn IXGBE_DCA_RXCTRL(i: usize) -> usize {
    IxgbeArrayRegs::DCA_RXCTRL as usize + (i * 0x40)
}
const fn IXGBE_TXPBTHRESH(i: usize) -> usize {
    IxgbeArrayRegs::TXPBTHRESH as usize + (i * 4)
}
const fn IXGBE_QPTC(i: usize) -> usize {
    IxgbeArrayRegs::QPTC as usize + (i * 0x40)
}
const fn IXGBE_EITR(i: usize) -> usize {
    IxgbeArrayRegs::EITR as usize + (i * 4)
}
const fn IXGBE_RXDCTL(i: usize) -> usize {
    IxgbeArrayRegs::RXDCTL as usize + (i * 0x40)
}

impl Intel8259x {
    /// Returns an initialized `Intel8259x` on success.
    pub fn new(bar: Box<dyn BarRegion>) -> Result<Self> {
        #[rustfmt::skip]
        let mut module = Intel8259x {
            receive_buffer: [
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
                allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            ],
            receive_ring: allocate_dma()?,
            receive_index: 0,
            transmit_ring: allocate_dma()?,
            transmit_ring_free: 512,
            transmit_index: 0,
            transmit_clean_index: 0,
            next_id: 0,
            bar,
            counter: 0,
            gcounter: 0,
        };

        println!("Calling module.init for ixgbe");
        module.init();
        //module.enable_loopback();

        println!("Module initialized");
        Ok(module)
    }

    pub fn read_reg(&self, register:IxgbeRegs) -> u32 {
        self.bar.read_reg32(register as usize)
    }

    fn read_reg_idx(&self, offset: usize) -> u32 {
        self.bar.read_reg32(offset)
    }

    fn write_reg(&self, register: IxgbeRegs, val: u32) {
        self.bar.write_reg32(register as usize, val);
    }

    pub fn write_reg_idx(&self, offset: usize, val: u32) {
        self.bar.write_reg32(offset, val);
    }

    fn wait_clear_reg(&self, register: IxgbeRegs, value: u32) {
        loop {
            let current = self.read_reg(register);
            if (current & value) == 0 {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn wait_write_reg(&self, register: IxgbeRegs, value: u32) {
        loop {
            let current = self.read_reg(register);
            if (current & value) == value {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn wait_write_reg_idx(&self, offset: usize, value: u32) {
        loop {
            let current = self.bar.read_reg32(offset);
            if (current & value) == value {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn write_flag(&self, register: IxgbeRegs, flags: u32) {
        self.write_reg(register, self.read_reg(register) | flags);
    }

    fn write_flag_idx(&self, offset: usize, flags: u32) {
        self.bar.write_reg32(offset, self.bar.read_reg32(offset) | flags);
    }

    fn clear_flag(&self, register: IxgbeRegs, flags: u32) {
        self.write_reg(register, self.read_reg(register) & !flags);
    }

    fn clear_flag_idx(&self, offset: usize, flags: u32) {
        self.bar.write_reg32(offset, self.bar.read_reg32(offset) & !flags);
    }

    /// Clear all interrupt masks for all queues.
    fn clear_interrupts(&self) {
        // Clear interrupt mask
        self.write_reg(IxgbeRegs::EIMC, IXGBE_IRQ_CLEAR_MASK);
        self.read_reg(IxgbeRegs::EICR);
    }

    /// Disable all interrupts for all queues.
    fn disable_interrupts(&self) {
        // Clear interrupt mask to stop from interrupts being generated
        self.write_reg(IxgbeRegs::EIMS, 0x0000_0000);
        self.clear_interrupts();
    }

    /// Resets and initializes an ixgbe device.
    fn init(&mut self) {
        println!("Disable irqs");
        self.disable_interrupts();

        println!("Writing regs");
        self.write_reg(IxgbeRegs::CTRL, IXGBE_CTRL_PCIE_MASTER_DISABLE); 

        self.wait_clear_reg(IxgbeRegs::STATUS, IXGBE_STATUS_PCIE_MASTER_STATUS); 

        // section 4.6.3.2
        self.write_reg(IxgbeRegs::CTRL, IXGBE_CTRL_RST_MASK);

        self.wait_clear_reg(IxgbeRegs::CTRL, IXGBE_CTRL_RST_MASK);
        println!("Sleep");
        sys_ns_loopsleep(ONE_MS_IN_NS * 100);

        println!("resume after Sleep");
        // section 4.6.3.1 - disable interrupts again after reset
        self.disable_interrupts();


        println!("No snoop disable bit");
        // check for no snoop disable bit
        let ctrl_ext = self.read_reg(IxgbeRegs::CTRLEXT);
        if (ctrl_ext & IXGBE_CTRL_EXT_NS_DIS) == 0 {
            self.write_reg(IxgbeRegs::CTRLEXT, ctrl_ext | IXGBE_CTRL_EXT_NS_DIS);
        }
        self.write_reg(IxgbeRegs::CTRLEXT, IXGBE_CTRL_EXT_DRV_LOAD);

        self.write_reg(IxgbeRegs::CTRLEXT, IXGBE_CTRL_EXT_DRV_LOAD);

        let mac = self.get_mac_addr();

        println!("initializing device");
        println!(
            "   - MAC: {:>02X}:{:>02X}:{:>02X}:{:>02X}:{:>02X}:{:>02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );

        /* 
        let _ = setcfg(
            "mac",
            &format!(
                "{:>02X}-{:>02X}-{:>02X}-{:>02X}-{:>02X}-{:>02X}\n",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            ),
        );*/

        // section 4.6.3 - wait for EEPROM auto read completion
        self.wait_write_reg(IxgbeRegs::EEC, IXGBE_EEC_ARD);

        // section 4.6.3 - wait for dma initialization done
        self.wait_write_reg(IxgbeRegs::RDRXCTL, IXGBE_RDRXCTL_DMAIDONE);

        // section 4.6.4 - initialize link (auto negotiation)
        self.init_link();

        // section 4.6.5 - statistical counters
        // reset-on-read registers, just read them once
        self.reset_stats();

        // section 4.6.7 - init rx
        self.init_rx();

        // section 4.6.8 - init tx
        self.init_tx();

        // start a single receive queue/ring
        self.start_rx_queue(0);

        // start a single transmit queue/ring
        self.start_tx_queue(0);

        // section 4.6.3.9 - enable interrupts
        //self.enable_msix_interrupt(0);

        // enable promisc mode by default to make testing easier
        self.set_promisc(true);

        // wait some time for the link to come up
        self.wait_for_link();

        self.dump_all_regs();

        // sleep for 10 seconds. Just stabilize the hardware
        // Well. this ugliness costed us two days of debugging.
        sys_ns_loopsleep(ONE_MS_IN_NS * 1000 * 3);
    }

    /// Returns the mac address of this device.
    pub fn get_mac_addr(&self) -> [u8; 6] {
        let low = self.bar.read_reg32(IXGBE_RAL(0));
        let high = self.bar.read_reg32(IXGBE_RAH(0));

        [
            (low & 0xff) as u8,
            (low >> 8 & 0xff) as u8,
            (low >> 16 & 0xff) as u8,
            (low >> 24) as u8,
            (high & 0xff) as u8,
            (high >> 8 & 0xff) as u8,
        ]
    }

    /// Sets the mac address of this device.
    #[allow(dead_code)]
    pub fn set_mac_addr(&self, mac: [u8; 6]) {
        let low: u32 = u32::from(mac[0])
            + (u32::from(mac[1]) << 8)
            + (u32::from(mac[2]) << 16)
            + (u32::from(mac[3]) << 24);
        let high: u32 = u32::from(mac[4]) + (u32::from(mac[5]) << 8);


        self.write_reg_idx(IXGBE_RAL(0), low as u32);
        self.write_reg_idx(IXGBE_RAH(0), high as u32);
    }

    // see section 4.6.4
    /// Initializes the link of this device.
    fn init_link(&self) {
        // link auto-configuration register should already be set correctly, we're resetting it anyway
        self.write_reg(
            IxgbeRegs::AUTOC,
            (self.read_reg(IxgbeRegs::AUTOC) & !IXGBE_AUTOC_LMS_MASK) | IXGBE_AUTOC_LMS_10G_SERIAL,
        );
        self.write_reg(
            IxgbeRegs::AUTOC,
            (self.read_reg(IxgbeRegs::AUTOC) & !IXGBE_AUTOC_10G_PMA_PMD_MASK) | IXGBE_AUTOC_10G_XAUI,
        );
        // negotiate link
        self.write_flag(IxgbeRegs::AUTOC, IXGBE_AUTOC_AN_RESTART);
        // datasheet wants us to wait for the link here, but we can continue and wait afterwards
    }

    /// Resets the stats of this device.
    fn reset_stats(&self) {
        self.read_reg(IxgbeRegs::GPRC);
        self.read_reg(IxgbeRegs::GPTC);
        self.read_reg(IxgbeRegs::GORCL);
        self.read_reg(IxgbeRegs::GORCH);
        self.read_reg(IxgbeRegs::GOTCL);
        self.read_reg(IxgbeRegs::GOTCH);
    }

    // sections 4.6.7
    /// Initializes the rx queues of this device.
    fn init_rx(&mut self) {
        // disable rx while re-configuring it
        self.clear_flag(IxgbeRegs::RXCTRL, IXGBE_RXCTRL_RXEN);

        // section 4.6.11.3.4 - allocate all queues and traffic to PB0
        self.bar.write_reg32(IXGBE_RXPBSIZE(0), IXGBE_RXPBSIZE_128KB);

        for i in 1..8 {
            self.bar.write_reg32(IXGBE_RXPBSIZE(i), 0);
        }

        // enable CRC offloading
        self.write_flag(IxgbeRegs::HLREG0, IXGBE_HLREG0_RXCRCSTRP);
        self.write_flag(IxgbeRegs::RDRXCTL, IXGBE_RDRXCTL_CRCSTRIP);

        // accept broadcast packets
        self.write_flag(IxgbeRegs::FCTRL, IXGBE_FCTRL_BAM);

        // configure a single receive queue/ring
        let i: usize = 0;

        // enable advanced rx descriptors
        self.bar.write_reg32(
            IXGBE_SRRCTL(i),
            (self.bar.read_reg32(IXGBE_SRRCTL(i)) & !IXGBE_SRRCTL_DESCTYPE_MASK)
                | IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF,
        );

        // let nic drop packets if no rx descriptor is available instead of buffering them
        self.write_flag_idx(IXGBE_SRRCTL(i), IXGBE_SRRCTL_DROP_EN);

        self.write_reg_idx(IXGBE_RDBAL(i), self.receive_ring.physical() as u32);

        self.write_reg_idx(IXGBE_RDBAH(i), (self.receive_ring.physical() >> 32) as u32);

        self.write_reg_idx(IXGBE_RDLEN(i),
            (self.receive_ring.len() * mem::size_of::<ixgbe_adv_rx_desc>()) as u32,
        );

        // set ring to empty at start
        self.write_reg_idx(IXGBE_RDH(i), 0);
        self.write_reg_idx(IXGBE_RDT(i), 0);

        // last sentence of section 4.6.7 - set some magic bits
        self.write_flag(IxgbeRegs::CTRLEXT, IXGBE_CTRL_EXT_NS_DIS);

        // probably a broken feature, this flag is initialized with 1 but has to be set to 0
        self.clear_flag_idx(IXGBE_DCA_RXCTRL(i), 1 << 12);

        // start rx
        self.write_flag(IxgbeRegs::RXCTRL, IXGBE_RXCTRL_RXEN);
    }

    fn enable_loopback(&self) {
        self.write_flag(IxgbeRegs::HLREG0, IXGBE_HLREG0_LPBK);
    }

    // section 4.6.8
    /// Initializes the tx queues of this device.
    fn init_tx(&mut self) {
        // crc offload and small packet padding
        self.write_flag(IxgbeRegs::HLREG0, IXGBE_HLREG0_TXCRCEN | IXGBE_HLREG0_TXPADEN);

        // section 4.6.11.3.4 - set default buffer size allocations
        self.write_reg_idx(IXGBE_TXPBSIZE(0), IXGBE_TXPBSIZE_40KB);
        for i in 1..8 {
            self.write_reg_idx(IXGBE_TXPBSIZE(i), 0);
        }

        self.write_reg_idx(IXGBE_TXPBTHRESH(0), 0xA0);

        for i in 1..8 {
            self.write_reg_idx(IXGBE_TXPBTHRESH(i), 0);
        }

        // required when not using DCB/VTd
        self.write_reg(IxgbeRegs::DTXMXSZRQ, 0xffff);
        self.clear_flag(IxgbeRegs::RTTDCS, IXGBE_RTTDCS_ARBDIS);

        // configure a single transmit queue/ring
        let i: usize = 0;

        // section 7.1.9 - setup descriptor ring

        self.write_reg_idx(IXGBE_TDBAL(i),
                                self.transmit_ring.physical() as u32);
        self.write_reg_idx(IXGBE_TDBAH(i),
                               (self.transmit_ring.physical() >> 32) as u32);

        println!("tx ring {} phys addr: {:#x}", i, self.transmit_ring.physical());
        self.write_reg_idx(IXGBE_TDLEN(i),
            (self.transmit_ring.len() * mem::size_of::<ixgbe_adv_tx_desc>()) as u32
        );

        // descriptor writeback magic values, important to get good performance and low PCIe overhead
        // see 7.2.3.4.1 and 7.2.3.5 for an explanation of these values and how to find good ones
        // we just use the defaults from DPDK here, but this is a potentially interesting point for optimizations
        //let mut txdctl = self.read_reg_idx(IxgbeArrayRegs::Txdctl, i);
        // there are no defines for this in ixgbe.rs for some reason
        // pthresh: 6:0, hthresh: 14:8, wthresh: 22:16
        //txdctl &= !(0x3F | (0x3F << 8) | (0x3F << 16));
        //txdctl |= 36 | (8 << 8) | (4 << 16);

        let mut txdctl = 0;
        txdctl |= 8 << 16;

        txdctl |= (1 << 8) | 32;

        self.write_reg_idx(IXGBE_TXDCTL(i), txdctl);

        // final step: enable DMA
        self.write_reg(IxgbeRegs::DMATXCTL, IXGBE_DMATXCTL_TE);
    }

    /// Returns the link speed of this device.
    fn get_link_speed(&self) -> u16 {
        let speed = self.read_reg(IxgbeRegs::LINKS);
        if (speed & IXGBE_LINKS_UP) == 0 {
            return 0;
        }
        match speed & IXGBE_LINKS_SPEED_82599 {
            IXGBE_LINKS_SPEED_100_82599 => 100,
            IXGBE_LINKS_SPEED_1G_82599 => 1000,
            IXGBE_LINKS_SPEED_10G_82599 => 10000,
            _ => 0,
        }
    }

    /// Sets the rx queues` descriptors and enables the queues.
    ///
    /// # Panics
    /// Panics if length of `self.receive_ring` is not a power of 2.
    fn start_rx_queue(&mut self, queue_id: u16) {
        if self.receive_ring.len() & (self.receive_ring.len() - 1) != 0 {
            panic!("number of receive queue entries must be a power of 2");
        }

        for i in 0..self.receive_ring.len() {
            unsafe {
                self.receive_ring[i].read.pkt_addr = self.receive_buffer[i].physical() as u64;
                self.receive_ring[i].read.hdr_addr = 0;
            }
        }

        // enable queue and wait if necessary
        self.write_flag_idx(IXGBE_RXDCTL(usize::from(queue_id)),
                                        IXGBE_RXDCTL_ENABLE);
        self.wait_write_reg_idx(IXGBE_RXDCTL(usize::from(queue_id)),
                                        IXGBE_RXDCTL_ENABLE);

        // rx queue starts out full
        self.write_reg_idx(IXGBE_RDH(usize::from(queue_id)), 0);

        // was set to 0 before in the init function
        self.write_reg_idx(
            IXGBE_RDT(usize::from(queue_id)),
            (self.receive_ring.len() - 1) as u32
        );
    }

    /// Enables the tx queues.
    ///
    /// # Panics
    /// Panics if length of `self.transmit_ring` is not a power of 2.
    fn start_tx_queue(&mut self, queue_id: u16) {
        if self.transmit_ring.len() & (self.transmit_ring.len() - 1) != 0 {
            panic!("number of receive queue entries must be a power of 2");
        }

        /*
        for i in 0..self.transmit_ring.len() {
            unsafe {
                self.transmit_ring[i].read.buffer_addr = self.transmit_buffer[i].physical() as u32;
            }
        }*/

        // tx queue starts out empty
        self.write_reg_idx(IXGBE_TDH(usize::from(queue_id)), 0);
        self.write_reg_idx(IXGBE_TDT(usize::from(queue_id)), 0);

        // enable queue and wait if necessary
        self.write_flag_idx(IXGBE_TXDCTL(usize::from(queue_id)),
                                            IXGBE_TXDCTL_ENABLE);
        self.wait_write_reg_idx(IXGBE_TXDCTL(usize::from(queue_id)),
                                            IXGBE_TXDCTL_ENABLE);
    }

    /// Enables or disables promisc mode of this device.
    fn set_promisc(&self, enabled: bool) {
        if enabled {
            self.write_flag(IxgbeRegs::FCTRL, IXGBE_FCTRL_MPE | IXGBE_FCTRL_UPE);
        } else {
            self.clear_flag(IxgbeRegs::FCTRL, IXGBE_FCTRL_MPE | IXGBE_FCTRL_UPE);
        }
    }

    /// Waits for the link to come up.
    fn wait_for_link(&self) {
        println!("   - waiting for link");
        let mut speed = self.get_link_speed();
        let mut count = 0;
        while speed == 0 && count < 100 {
            count = count + 1;
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
            speed = self.get_link_speed();
        }
        println!("   - link speed is {} Mbit/s", self.get_link_speed());
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<Option<usize>> {
        if self.transmit_ring_free == 0 {
            loop {
                let desc = unsafe {
                    &*(self.transmit_ring.as_ptr().add(self.transmit_clean_index)
                        as *const ixgbe_adv_tx_desc)
                };

                if (unsafe { desc.wb.status } & IXGBE_ADVTXD_STAT_DD as u32) != 0 {
                    self.transmit_clean_index =
                        wrap_ring(self.transmit_clean_index, self.transmit_ring.len());
                    self.transmit_ring_free += 1;
                } else if self.transmit_ring_free > 0 {
                    break;
                }

                if self.transmit_ring_free >= self.transmit_ring.len() {
                    break;
                }
            }
        }

        let desc = unsafe {
            &mut *(self.transmit_ring.as_ptr().add(self.transmit_index) as *mut ixgbe_adv_tx_desc)
        };

        desc.read.buffer_addr = buf as *const _ as *const u64 as u64; 

        unsafe {
            desc.read.cmd_type_len = IXGBE_ADVTXD_DCMD_EOP
                | IXGBE_ADVTXD_DCMD_RS
                | IXGBE_ADVTXD_DCMD_IFCS
                | IXGBE_ADVTXD_DCMD_DEXT
                | IXGBE_ADVTXD_DTYP_DATA
                | buf.len() as u32;

            desc.read.olinfo_status = (buf.len() as u32) << IXGBE_ADVTXD_PAYLEN_SHIFT;
        }

        self.transmit_index = wrap_ring(self.transmit_index, self.transmit_ring.len());
        self.transmit_ring_free -= 1;

        self.write_reg_idx(IXGBE_TDT(0), self.transmit_index as u32);

        Ok(Some(0))
    }

    fn clean_tx_queue(&mut self) -> usize {
        let mut clean_index = self.transmit_clean_index;
        let cur_index = self.transmit_index;

        loop {
            let num_descriptors = self.transmit_ring.len();

            let status = unsafe {
                core::ptr::read_volatile(&(*self.transmit_ring.as_ptr().add(clean_index)).wb.status
                   as *const u32)
            };

            if (status & IXGBE_ADVTXD_STAT_DD) != 0 {
                clean_index = wrap_ring(clean_index, num_descriptors);
            } else {
                break;
            }
        }

        self.transmit_clean_index = clean_index;

        clean_index
    }

    /// Pops as many packets as possible from `packets` to put them into the device`s tx queue.
    pub fn tx_batch<T>(&mut self, packets: &Vec<UdpPacket<T>>) -> usize {
        let mut sent = 0;

        {
            let mut cur_index = self.transmit_index;
            let clean_index = self.clean_tx_queue();
            let num_descriptors = self.transmit_ring.len();

            for mut packet in packets {
                let mut pslice = packet.as_slice();

                let next_index = wrap_ring(cur_index, num_descriptors);

                if clean_index == next_index {
                    // tx queue of device is full, push packet back onto the
                    // queue of to-be-sent packets
                    //println!("No space in queue");
                    break;
                }

                //self.gcounter = self.gcounter.wrapping_add(1);

                // for debugging only
                /*unsafe {
                    let mut mpslice = packet as *const UdpPacket<T> as *mut u8;
                    *mpslice.add(PACKET_SIZE - 4) = ((self.gcounter >> 24) as u8) & 0xFF;
                    *mpslice.add(PACKET_SIZE - 3) = ((self.gcounter >> 16) as u8) & 0xFF;
                    *mpslice.add(PACKET_SIZE - 2) = ((self.gcounter >> 8) as u8) & 0xFF;
                    *mpslice.add(PACKET_SIZE - 1) = ((self.gcounter >> 0) as u8) & 0xFF;
                }*/

                self.transmit_index = wrap_ring(self.transmit_index, num_descriptors);

                unsafe {

                    core::ptr::write_volatile(
                            &(*self.transmit_ring.as_ptr().add(cur_index)).read.buffer_addr as *const u64 as *mut u64,
                            pslice as *const _ as *const u64 as u64
                    );

                    core::ptr::write_volatile(
                            &(*self.transmit_ring.as_ptr().add(cur_index)).read.cmd_type_len as *const u32 as *mut u32,
                            IXGBE_ADVTXD_DCMD_EOP
                                    | IXGBE_ADVTXD_DCMD_RS
                                    | IXGBE_ADVTXD_DCMD_IFCS
                                    | IXGBE_ADVTXD_DCMD_DEXT
                                    | IXGBE_ADVTXD_DTYP_DATA
                                    | pslice.len() as u32,
                    );

                    core::ptr::write_volatile(
                            &(*self.transmit_ring.as_ptr().add(cur_index)).read.olinfo_status as *const u32 as *mut u32,
                            (pslice.len() as u32) << IXGBE_ADVTXD_PAYLEN_SHIFT,
                    );
                }

                cur_index = next_index;
                sent += 1;
            }
        }
 
        //println!("updating tail {}", self.transmit_index);
        if sent > 0 {
            self.write_reg_idx(IXGBE_TDT(0), self.transmit_index as u32);
        }
        //println!("wrote {} packets", sent);

        sent
    }

    fn set_ivar(&mut self, direction: i8, queue_id: u16, mut msix_vector: u8) {
        let index = ((16 * (queue_id & 1)) as i16 + i16::from(8 * direction)) as u32;

        msix_vector |= IXGBE_IVAR_ALLOC_VAL as u8;

        let mut ivar = self.read_reg_idx(IXGBE_IVAR(usize::from(queue_id >> 1)));
        ivar &= !(0xFF << index);
        ivar |= u32::from(msix_vector << index);

        self.write_reg_idx(IXGBE_IVAR(usize::from(queue_id >> 1)), ivar);
    }


    /// Enable MSI-X interrupt for a queue.
    fn enable_msix_interrupt(&mut self, queue_id: u16) {
        // Step 1: The software driver associates between interrupt causes and MSI-X vectors and the
        //throttling timers EITR[n] by programming the IVAR[n] and IVAR_MISC registers.
        let mut gpie: u32 = self.read_reg(IxgbeRegs::GPIE);
        gpie |= IXGBE_GPIE_MSIX_MODE | IXGBE_GPIE_PBA_SUPPORT | IXGBE_GPIE_EIAME;

        self.write_reg(IxgbeRegs::GPIE, gpie as u32);

        self.set_ivar(0, queue_id, queue_id as u8);

        // Step 2: Program SRRCTL[n].RDMTS (per receive queue) if software uses the receive
        // descriptor minimum threshold interrupt

        // Step 3: The EIAC[n] registers should be set to auto clear for transmit and receive interrupt
        // causes (for best performance). The EIAC bits that control the other and TCP timer
        // interrupt causes should be set to 0b (no auto clear).
        self.write_reg(IxgbeRegs::EIAC, IXGBE_EICR_RTX_QUEUE);

        self.write_reg_idx(IXGBE_EITR(queue_id as usize), 0x028);
        // Step 4: Set the auto mask in the EIAM register according to the preferred mode of operation.

        // Step 5: Set the interrupt throttling in EITR[n] and GPIE according to the preferred mode of operation.

        // Step 6: Software enables the required interrupt causes by setting the EIMS register
        let mut mask: u32 = self.read_reg(IxgbeRegs::EIMS) as u32;
        mask |= 1 << queue_id;

        self.write_reg(IxgbeRegs::EIMS, mask as u32);
    }

    pub fn dump_stats(&self) {
        println!("Ixgbe statistics:");
        let mut string = format!("Stats regs:\n\tGPRC {:08X} GPTC {:08X}\n\tGORCL {:08X} GORCH {:08X}\n\tGOTCL {:08X} GOTCH {:08X}\n\tTXDGPC {:08X} TXDGBCH {:08X} TXDGBCL {:08X} QPTC(0) {:08X}\n",
                                self.read_reg(IxgbeRegs::GPRC) as u32,
                                self.read_reg(IxgbeRegs::GPTC) as u32,
                                self.read_reg(IxgbeRegs::GORCL) as u32,
                                self.read_reg(IxgbeRegs::GORCH) as u32,
                                self.read_reg(IxgbeRegs::GOTCL) as u32,
                                self.read_reg(IxgbeRegs::GOTCH) as u32,
                                self.read_reg(IxgbeRegs::TXDGPC) as u32,
                                self.read_reg(IxgbeRegs::TXDGBCH) as u32,
                                self.read_reg(IxgbeRegs::TXDGBCL) as u32,
                                self.read_reg_idx(IXGBE_QPTC(0)) as u32,
                                );
        print!("{}", string);
    }

    pub fn dump_all_regs(&self) { 
        let mut string = format!("Interrupt regs:\n\tEICR: {:08X} EIMS: {:08X} EIMC: {:08X}\n\tEITR {:08X} GPIE {:08X}\n\tIVAR(0) {:08X}\n",
                    self.read_reg(IxgbeRegs::EICR) as u32,
                    self.read_reg(IxgbeRegs::EIMS) as u32,
                    self.read_reg(IxgbeRegs::EIMC) as u32,
                    self.read_reg_idx(IXGBE_EITR(0)) as u32,
                    self.read_reg(IxgbeRegs::GPIE) as u32,
                    self.read_reg_idx(IXGBE_IVAR(0)) as u32, 
                    );

        string.push_str(&format!("Control regs:\n\tCTRL {:08X} CTRL_EXT {:08X}\n",
                                 self.read_reg(IxgbeRegs::CTRL) as u32,
                                 self.read_reg(IxgbeRegs::CTRLEXT) as u32, 
                                 ));

        string.push_str(&format!("EEPROM regs:\n\tEEC_ARD {:08X}\n",
                                 self.read_reg(IxgbeRegs::EEC) as u32));

        string.push_str(&format!("AUTOC {:08X}\n",
                                 self.read_reg(IxgbeRegs::AUTOC) as u32));

        string.push_str(&format!("Receive regs:\n\tRDRXCTRL {:08X} RXCTRL {:08X} RXPBSIZE(0): {:08X}\n\tHLREG0 {:08X} FCTRL {:08X}\n\tSRRCTL(0) {:08X} RDBAL(0) {:08X} RDBAH(0) {:08X} RDLEN(0) {:08X}\nRDH(0) {:08X} RDT(0) {:08X}\n",
                                 self.read_reg(IxgbeRegs::RDRXCTL) as u32,
                                 self.read_reg(IxgbeRegs::RXCTRL) as u32,
                                 self.read_reg_idx(IXGBE_RXPBSIZE(0)) as u32,
                                 self.read_reg(IxgbeRegs::HLREG0) as u32,
                                 self.read_reg(IxgbeRegs::FCTRL) as u32,
                                 self.read_reg_idx(IXGBE_SRRCTL(0)) as u32,
                                 self.read_reg_idx(IXGBE_RDBAL(0)) as u32,
                                 self.read_reg_idx(IXGBE_RDBAH(0)) as u32,
                                 self.read_reg_idx(IXGBE_RDLEN(0)) as u32,
                                 self.read_reg_idx(IXGBE_RDH(0)) as u32,
                                 self.read_reg_idx(IXGBE_RDT(0)) as u32,
                                 ));

        string.push_str(&format!("Transmit regs:\n\tTXDCTL(0) {:08X} TXPBSIZE(0): {:08X}\n\tDTXMSSZRQ {:08X} RTTDCS {:08X}\n\tDMATXCTL: {:08X} TDBAL(0) {:08X} TDBAH(0) {:08X} TDLEN(0) {:08X}\n\tTDH(0) {:08X} TDT(0) {:08X}\n",
                                 self.read_reg_idx(IXGBE_TXDCTL(0)) as u32,
                                 self.read_reg_idx(IXGBE_TXPBSIZE(0)) as u32,
                                 self.read_reg(IxgbeRegs::DTXMXSZRQ) as u32,
                                 self.read_reg(IxgbeRegs::RTTDCS) as u32,
                                 self.read_reg(IxgbeRegs::DMATXCTL) as u32,
                                 self.read_reg_idx(IXGBE_TDBAL(0)) as u32,
                                 self.read_reg_idx(IXGBE_TDBAH(0)) as u32,
                                 self.read_reg_idx(IXGBE_TDLEN(0)) as u32,
                                 self.read_reg_idx(IXGBE_TDH(0)) as u32,
                                 self.read_reg_idx(IXGBE_TDT(0)) as u32,
                                 ));
        string.push_str(&format!("Stats regs:\n\tGPRC {:08X} GPTC {:08X}\n\tGORCL {:08X} GORCH {:08X}\n\tGOTCL {:08X} GOTCH {:08X}\n\tTXDGPC {:08X} TXDGBCH {:08X} TXDGBCL {:08X} QPTC(0) {:08X}\n",
                                self.read_reg(IxgbeRegs::GPRC) as u32,
                                self.read_reg(IxgbeRegs::GPTC) as u32,
                                self.read_reg(IxgbeRegs::GORCL) as u32,
                                self.read_reg(IxgbeRegs::GORCH) as u32,
                                self.read_reg(IxgbeRegs::GOTCL) as u32,
                                self.read_reg(IxgbeRegs::GOTCH) as u32,
                                self.read_reg(IxgbeRegs::TXDGPC) as u32,
                                self.read_reg(IxgbeRegs::TXDGBCH) as u32,
                                self.read_reg(IxgbeRegs::TXDGBCL) as u32,
                                self.read_reg_idx(IXGBE_QPTC(0)) as u32,
                                ));
        print!("{}", string);
        self.dump_stats();
    }
}
