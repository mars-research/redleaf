#![no_std]

use alloc::boxed::Box;
use alloc::vec::Vec;
use ixgbe::IxgbeBarRegion;
use crate::dma::Dma;
use crate::ixgbe_desc::*;
use crate::Result;
use ixgbe::{IxgbeRegs, IxgbeArrayRegs};
use console::println;
use core::mem;
use libsyscalls::time::sys_ns_sleep;

const ONE_MS_IN_NS: u64 = 100_0000;

pub struct Intel8259x {
    //base: usize,
    //size: usize,
    receive_buffer: [Dma<[u8; 16384]>; 32],
    receive_ring: Dma<[ixgbe_adv_rx_desc; 32]>,
    receive_index: usize,
    transmit_buffer: [Dma<[u8; 16384]>; 32],
    transmit_ring: Dma<[ixgbe_adv_tx_desc; 32]>,
    transmit_ring_free: usize,
    transmit_index: usize,
    transmit_clean_index: usize,
    next_id: usize,
    bar: Box<dyn IxgbeBarRegion>,
    //pub handles: BTreeMap<usize, usize>,
}

impl Intel8259x {
    /// Returns an initialized `Intel8259x` on success.
    //pub fn new(base: usize, size: usize) -> Result<Self> {
    pub fn new(bar: Box<dyn IxgbeBarRegion>) -> Result<Self> {
        #[rustfmt::skip]
        let mut module = Intel8259x {
            receive_buffer: [
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
            ],
            receive_ring: Dma::zeroed()?,
            transmit_buffer: [
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
                Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?, Dma::zeroed()?,
            ],
            receive_index: 0,
            transmit_ring: Dma::zeroed()?,
            transmit_ring_free: 32,
            transmit_index: 0,
            transmit_clean_index: 0,
            next_id: 0,
            bar,
        };

       // module.init();

        Ok(module)
    }

    fn wait_clear_reg(&self, register: IxgbeRegs, value: u64) {
        loop {
            let current = self.bar.read_reg(register);
            if (current & value) == 0 {
                break;
            }
            sys_ns_sleep(ONE_MS_IN_NS * 100);
        }
    }

    fn wait_write_reg(&self, register: IxgbeRegs, value: u64) {
        loop {
            let current = self.bar.read_reg(register);
            if (current & value) == value {
                break;
            }
            sys_ns_sleep(ONE_MS_IN_NS * 100);
        }
    }

    fn wait_write_reg_idx(&self, register: IxgbeArrayRegs, idx: u64, value: u64) {
        loop {
            let current = self.bar.read_reg_idx(register, idx);
            if (current & value) == value {
                break;
            }
            sys_ns_sleep(ONE_MS_IN_NS * 100);
        }
    }

    fn write_flag(&self, register: IxgbeRegs, flags: u64) {
        self.bar.write_reg(register, self.bar.read_reg(register) | flags);
    }

    fn write_flag_idx(&self, register: IxgbeArrayRegs, idx: u64, flags: u64) {
        self.bar.write_reg_idx(register, idx, self.bar.read_reg_idx(register, idx) | flags);
    }

    fn clear_flag(&self, register: IxgbeRegs, flags: u64) {
        self.bar.write_reg(register, self.bar.read_reg(register) & !flags);
    }

    fn clear_flag_idx(&self, register: IxgbeArrayRegs, idx: u64, flags: u64) {
        self.bar.write_reg_idx(register, idx, self.bar.read_reg_idx(register, idx) & !flags);
    }


    /// Resets and initializes an ixgbe device.
    fn init(&mut self) {
        // section 4.6.3.1 - disable all interrupts
        self.bar.write_reg(IxgbeRegs::Eimc, 0x7fff_ffff);

        // section 4.6.3.2
        self.bar.write_reg(IxgbeRegs::Ctrl, IXGBE_CTRL_RST_MASK);

        self.wait_clear_reg(IxgbeRegs::Ctrl, IXGBE_CTRL_RST_MASK);
        sys_ns_sleep(ONE_MS_IN_NS * 100);

        // section 4.6.3.1 - disable interrupts again after reset
        self.bar.write_reg(IxgbeRegs::Eimc, 0x7fff_ffff);

        let mac = self.get_mac_addr();

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
        self.wait_write_reg(IxgbeRegs::Eec, IXGBE_EEC_ARD);

        // section 4.6.3 - wait for dma initialization done
        self.wait_write_reg(IxgbeRegs::Rdrxctl, IXGBE_RDRXCTL_DMAIDONE);

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
        //self.wait_for_link();
    }

    /// Returns the mac address of this device.
    pub fn get_mac_addr(&self) -> [u8; 6] {
        let low = self.bar.read_reg_idx(IxgbeArrayRegs::Ral, 0);
        let high = self.bar.read_reg_idx(IxgbeArrayRegs::Rah, 0);

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


        self.bar.write_reg_idx(IxgbeArrayRegs::Ral, 0, low as u64);
        self.bar.write_reg_idx(IxgbeArrayRegs::Rah, 0, high as u64);
    }

    // see section 4.6.4
    /// Initializes the link of this device.
    fn init_link(&self) {
        // link auto-configuration register should already be set correctly, we're resetting it anyway
        self.bar.write_reg(
            IxgbeRegs::Autoc,
            (self.bar.read_reg(IxgbeRegs::Autoc) & !IXGBE_AUTOC_LMS_MASK) | IXGBE_AUTOC_LMS_10G_SERIAL,
        );
        self.bar.write_reg(
            IxgbeRegs::Autoc,
            (self.bar.read_reg(IxgbeRegs::Autoc) & !IXGBE_AUTOC_10G_PMA_PMD_MASK) | IXGBE_AUTOC_10G_XAUI,
        );
        // negotiate link
        self.write_flag(IxgbeRegs::Autoc, IXGBE_AUTOC_AN_RESTART);
        // datasheet wants us to wait for the link here, but we can continue and wait afterwards
    }

    /// Resets the stats of this device.
    fn reset_stats(&self) {
        self.bar.read_reg(IxgbeRegs::Gprc);
        self.bar.read_reg(IxgbeRegs::Gptc);
        self.bar.read_reg(IxgbeRegs::Gorcl);
        self.bar.read_reg(IxgbeRegs::Gorch);
        self.bar.read_reg(IxgbeRegs::Gotcl);
        self.bar.read_reg(IxgbeRegs::Gotch);
    }

    // sections 4.6.7
    /// Initializes the rx queues of this device.
    fn init_rx(&mut self) {
        // disable rx while re-configuring it
        self.clear_flag(IxgbeRegs::Rxctrl, IXGBE_RXCTRL_RXEN);

        // section 4.6.11.3.4 - allocate all queues and traffic to PB0
        self.bar.write_reg_idx(IxgbeArrayRegs::Rxpbsize, 0, IXGBE_RXPBSIZE_128KB);

        for i in 1..8 {
            self.bar.write_reg_idx(IxgbeArrayRegs::Rxpbsize, i, 0);
        }

        // enable CRC offloading
        self.write_flag(IxgbeRegs::Hlreg0, IXGBE_HLREG0_RXCRCSTRP);
        self.write_flag(IxgbeRegs::Rdrxctl, IXGBE_RDRXCTL_CRCSTRIP);

        // accept broadcast packets
        self.write_flag(IxgbeRegs::Fctrl, IXGBE_FCTRL_BAM);

        // configure a single receive queue/ring
        let i: u64 = 0;

        // enable advanced rx descriptors
        self.bar.write_reg_idx(
            IxgbeArrayRegs::Srrctl, i,
            (self.bar.read_reg_idx(IxgbeArrayRegs::Srrctl, i) & !IXGBE_SRRCTL_DESCTYPE_MASK)
                | IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF,
        );
        // let nic drop packets if no rx descriptor is available instead of buffering them
        self.write_flag_idx(IxgbeArrayRegs::Srrctl, i, IXGBE_SRRCTL_DROP_EN);

        self.bar.write_reg_idx(IxgbeArrayRegs::Rdbal, i, self.receive_ring.physical() as u64);

        self.bar.write_reg_idx(IxgbeArrayRegs::Rdbah, i, (self.receive_ring.physical() >> 32) as u64);
        self.bar.write_reg_idx(
            IxgbeArrayRegs::Rdlen, i,
            (self.receive_ring.len() * mem::size_of::<ixgbe_adv_rx_desc>()) as u64,
        );

        // set ring to empty at start
        self.bar.write_reg_idx(IxgbeArrayRegs::Rdh, i, 0);
        self.bar.write_reg_idx(IxgbeArrayRegs::Rdt, i, 0);

        // last sentence of section 4.6.7 - set some magic bits
        self.write_flag(IxgbeRegs::Ctrlext, IXGBE_CTRL_EXT_NS_DIS);

        // probably a broken feature, this flag is initialized with 1 but has to be set to 0
        self.clear_flag_idx(IxgbeArrayRegs::DcaRxctrl, i, 1 << 12);

        // start rx
        self.write_flag(IxgbeRegs::Rxctrl, IXGBE_RXCTRL_RXEN);
    }

    // section 4.6.8
    /// Initializes the tx queues of this device.
    fn init_tx(&mut self) {
        // crc offload and small packet padding
        self.write_flag(IxgbeRegs::Hlreg0, IXGBE_HLREG0_TXCRCEN | IXGBE_HLREG0_TXPADEN);

        // section 4.6.11.3.4 - set default buffer size allocations
        self.bar.write_reg_idx(IxgbeArrayRegs::Txpbsize, 0, IXGBE_TXPBSIZE_40KB);
        for i in 1..8 {
            self.bar.write_reg_idx(IxgbeArrayRegs::Txpbsize, i, 0);
        }

        // required when not using DCB/VTd
        self.bar.write_reg(IxgbeRegs::Dtxmxszrq, 0xffff);
        self.clear_flag(IxgbeRegs::Rttdcs, IXGBE_RTTDCS_ARBDIS);

        // configure a single transmit queue/ring
        let i: u64 = 0;

        // section 7.1.9 - setup descriptor ring

        self.bar.write_reg_idx(IxgbeArrayRegs::Tdbal, i,
                                self.transmit_ring.physical() as u64);
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdbah, i,
                               (self.transmit_ring.physical() >> 32) as u64);

        self.bar.write_reg_idx(IxgbeArrayRegs::Tdlen, i,
            (self.transmit_ring.len() * mem::size_of::<ixgbe_adv_tx_desc>()) as u64
        );

        // descriptor writeback magic values, important to get good performance and low PCIe overhead
        // see 7.2.3.4.1 and 7.2.3.5 for an explanation of these values and how to find good ones
        // we just use the defaults from DPDK here, but this is a potentially interesting point for optimizations
        let mut txdctl = self.bar.read_reg_idx(IxgbeArrayRegs::Txdctl, i);
        // there are no defines for this in ixgbe.rs for some reason
        // pthresh: 6:0, hthresh: 14:8, wthresh: 22:16
        txdctl &= !(0x3F | (0x3F << 8) | (0x3F << 16));
        txdctl |= 36 | (8 << 8) | (4 << 16);

        self.bar.write_reg_idx(IxgbeArrayRegs::Txdctl, i, txdctl);

        // final step: enable DMA
        self.bar.write_reg(IxgbeRegs::Dmatxctl, IXGBE_DMATXCTL_TE);
    }

    /// Returns the link speed of this device.
    fn get_link_speed(&self) -> u16 {
        let speed = self.bar.read_reg(IxgbeRegs::Links);
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
        self.write_flag_idx(IxgbeArrayRegs::Rxdctl, u64::from(queue_id),
                                        IXGBE_RXDCTL_ENABLE);
        self.wait_write_reg_idx(IxgbeArrayRegs::Rxdctl, u64::from(queue_id),
                                        IXGBE_RXDCTL_ENABLE);

        // rx queue starts out full
        self.bar.write_reg_idx(IxgbeArrayRegs::Rdh, u64::from(queue_id), 0);

        // was set to 0 before in the init function
        self.bar.write_reg_idx(
            IxgbeArrayRegs::Rdt,
            u64::from(queue_id),
            (self.receive_ring.len() - 1) as u64
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

        for i in 0..self.transmit_ring.len() {
            unsafe {
                self.transmit_ring[i].read.buffer_addr = self.transmit_buffer[i].physical() as u64;
            }
        }

        // tx queue starts out empty
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdh, u64::from(queue_id), 0);
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdt, u64::from(queue_id), 0);

        // enable queue and wait if necessary
        self.write_flag_idx(IxgbeArrayRegs::Txdctl, u64::from(queue_id),
                                            IXGBE_TXDCTL_ENABLE);
        self.wait_write_reg_idx(IxgbeArrayRegs::Txdctl, u64::from(queue_id),
                                            IXGBE_TXDCTL_ENABLE);
    }

    /// Enables or disables promisc mode of this device.
    fn set_promisc(&self, enabled: bool) {
        if enabled {
            self.write_flag(IxgbeRegs::Fctrl, IXGBE_FCTRL_MPE | IXGBE_FCTRL_UPE);
        } else {
            self.clear_flag(IxgbeRegs::Fctrl, IXGBE_FCTRL_MPE | IXGBE_FCTRL_UPE);
        }
    }

    /// Waits for the link to come up.
    fn wait_for_link(&self) {
        println!("   - waiting for link");
        let mut speed = self.get_link_speed();
        let mut count = 0;
        while speed == 0 && count < 100 {
            count = count + 1;
            sys_ns_sleep(ONE_MS_IN_NS * 100);
            speed = self.get_link_speed();
        }
        println!("   - link speed is {} Mbit/s", self.get_link_speed());
    }
}
