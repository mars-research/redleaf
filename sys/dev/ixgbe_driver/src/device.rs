#![no_std]

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::ixgbe_desc::*;
use crate::Result;
use ixgbe::{IxgbeRegs, IxgbeArrayRegs, IxgbeBarRegion};
use ixgbe_device::IxgbeDevice;
use console::{println, print};
use core::{mem};
use libtime::sys_ns_loopsleep;
use alloc::format;
use protocol::UdpPacket;

const ONE_MS_IN_NS: u64 = 1_000_000 * 1;
const PACKET_SIZE: usize = 60;

pub struct Intel8259x {
    pub device: IxgbeDevice,
}

impl Intel8259x {
    /// Returns an initialized `Intel8259x` on success.
    pub fn new(bar: Box<dyn IxgbeBarRegion>) -> Result<Self> {
        #[rustfmt::skip]
        let mut module = Intel8259x {
            device: IxgbeDevice::new(bar),
        };

        println!("Calling module.init for ixgbe");
        module.init();
        //module.enable_loopback();

        println!("Module initialized");
        Ok(module)
    }

    fn read_reg(&self, register: IxgbeRegs) -> u64 {
        self.device.bar.read_reg(register)
    }

    fn read_reg_idx(&self, register: IxgbeArrayRegs, idx: u64) -> u64 {
        self.device.bar.read_reg_idx(register, idx)
    }

    fn write_reg(&self, register: IxgbeRegs, value: u64) {
        self.device.bar.write_reg(register, value);
    }

    fn write_reg_idx(&self, register: IxgbeArrayRegs, idx: u64, value: u64) {
        self.device.bar.write_reg_idx(register, idx, value);
    }



    fn wait_clear_reg(&self, register: IxgbeRegs, value: u64) {
        loop {
            let current = self.read_reg(register);
            if (current & value) == 0 {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn wait_write_reg(&self, register: IxgbeRegs, value: u64) {
        loop {
            let current = self.read_reg(register);
            if (current & value) == value {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn wait_write_reg_idx(&self, register: IxgbeArrayRegs, idx: u64, value: u64) {
        loop {
            let current = self.read_reg_idx(register, idx);
            if (current & value) == value {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn write_flag(&self, register: IxgbeRegs, flags: u64) {
        self.write_reg(register, self.read_reg(register) | flags);
    }

    fn write_flag_idx(&self, register: IxgbeArrayRegs, idx: u64, flags: u64) {
        self.write_reg_idx(register, idx, self.read_reg_idx(register, idx) | flags);
    }

    fn clear_flag(&self, register: IxgbeRegs, flags: u64) {
        self.write_reg(register, self.read_reg(register) & !flags);
    }

    fn clear_flag_idx(&self, register: IxgbeArrayRegs, idx: u64, flags: u64) {
        self.write_reg_idx(register, idx, self.read_reg_idx(register, idx) & !flags);
    }

    /// Clear all interrupt masks for all queues.
    fn clear_interrupts(&self) {
        // Clear interrupt mask
        self.write_reg(IxgbeRegs::Eimc, IXGBE_IRQ_CLEAR_MASK);
        self.read_reg(IxgbeRegs::Eicr);
    }

    /// Disable all interrupts for all queues.
    fn disable_interrupts(&self) {
        // Clear interrupt mask to stop from interrupts being generated
        self.write_reg(IxgbeRegs::Eims, 0x0000_0000);
        self.clear_interrupts();
    }

    /// Resets and initializes an ixgbe device.
    fn init(&mut self) {
        println!("Disable irqs");
        self.disable_interrupts();

        println!("Writing regs");
        self.write_reg(IxgbeRegs::Ctrl, IXGBE_CTRL_PCIE_MASTER_DISABLE);

        self.wait_clear_reg(IxgbeRegs::Status, IXGBE_STATUS_PCIE_MASTER_STATUS);

        // section 4.6.3.2
        self.write_reg(IxgbeRegs::Ctrl, IXGBE_CTRL_RST_MASK);

        self.wait_clear_reg(IxgbeRegs::Ctrl, IXGBE_CTRL_RST_MASK);
        println!("Sleep");
        sys_ns_loopsleep(ONE_MS_IN_NS * 100);

        println!("resume after Sleep");
        // section 4.6.3.1 - disable interrupts again after reset
        self.disable_interrupts();


        println!("No snoop disable bit");
        // check for no snoop disable bit
        let ctrl_ext = self.read_reg(IxgbeRegs::Ctrlext);
        if (ctrl_ext & IXGBE_CTRL_EXT_NS_DIS) == 0 {
            self.write_reg(IxgbeRegs::Ctrlext, ctrl_ext | IXGBE_CTRL_EXT_NS_DIS);
        }
        self.write_reg(IxgbeRegs::Ctrlext, IXGBE_CTRL_EXT_DRV_LOAD);

        self.write_reg(IxgbeRegs::Ctrlext, IXGBE_CTRL_EXT_DRV_LOAD);

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
        self.wait_for_link();

        self.dump_all_regs();

        // sleep for 10 seconds. Just stabilize the hardware
        // Well. this ugliness costed us two days of debugging.
        println!("Sleep for 15 seconds");
        sys_ns_loopsleep(ONE_MS_IN_NS * 1000 * 3);
        println!("Resuming sleep");
    }

    /// Returns the mac address of this device.
    pub fn get_mac_addr(&self) -> [u8; 6] {
        let low = self.read_reg_idx(IxgbeArrayRegs::Ral, 0);
        let high = self.read_reg_idx(IxgbeArrayRegs::Rah, 0);

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


        self.write_reg_idx(IxgbeArrayRegs::Ral, 0, low as u64);
        self.write_reg_idx(IxgbeArrayRegs::Rah, 0, high as u64);
    }

    // see section 4.6.4
    /// Initializes the link of this device.
    fn init_link(&self) {
        // link auto-configuration register should already be set correctly, we're resetting it anyway
        self.write_reg(
            IxgbeRegs::Autoc,
            (self.read_reg(IxgbeRegs::Autoc) & !IXGBE_AUTOC_LMS_MASK) | IXGBE_AUTOC_LMS_10G_SERIAL,
        );
        self.write_reg(
            IxgbeRegs::Autoc,
            (self.read_reg(IxgbeRegs::Autoc) & !IXGBE_AUTOC_10G_PMA_PMD_MASK) | IXGBE_AUTOC_10G_XAUI,
        );
        // negotiate link
        self.write_flag(IxgbeRegs::Autoc, IXGBE_AUTOC_AN_RESTART);
        // datasheet wants us to wait for the link here, but we can continue and wait afterwards
    }

    /// Resets the stats of this device.
    fn reset_stats(&self) {
        self.read_reg(IxgbeRegs::Gprc);
        self.read_reg(IxgbeRegs::Gptc);
        self.read_reg(IxgbeRegs::Gorcl);
        self.read_reg(IxgbeRegs::Gorch);
        self.read_reg(IxgbeRegs::Gotcl);
        self.read_reg(IxgbeRegs::Gotch);
    }

    // sections 4.6.7
    /// Initializes the rx queues of this device.
    fn init_rx(&mut self) {
        // disable rx while re-configuring it
        self.clear_flag(IxgbeRegs::Rxctrl, IXGBE_RXCTRL_RXEN);

        // section 4.6.11.3.4 - allocate all queues and traffic to PB0
        self.write_reg_idx(IxgbeArrayRegs::Rxpbsize, 0, IXGBE_RXPBSIZE_128KB);

        for i in 1..8 {
            self.write_reg_idx(IxgbeArrayRegs::Rxpbsize, i, 0);
        }

        // enable CRC offloading
        self.write_flag(IxgbeRegs::Hlreg0, IXGBE_HLREG0_RXCRCSTRP);
        self.write_flag(IxgbeRegs::Rdrxctl, IXGBE_RDRXCTL_CRCSTRIP);

        // accept broadcast packets
        self.write_flag(IxgbeRegs::Fctrl, IXGBE_FCTRL_BAM);

        // configure a single receive queue/ring
        let i: u64 = 0;

        // TODO: Manipulation of rx queue. Move this to trusted part
        self.device.init_rx();

        // last sentence of section 4.6.7 - set some magic bits
        self.write_flag(IxgbeRegs::Ctrlext, IXGBE_CTRL_EXT_NS_DIS);

        // probably a broken feature, this flag is initialized with 1 but has to be set to 0
        self.clear_flag_idx(IxgbeArrayRegs::DcaRxctrl, i, 1 << 12);

        // start rx
        self.write_flag(IxgbeRegs::Rxctrl, IXGBE_RXCTRL_RXEN);
    }

    fn enable_loopback(&self) {
        self.write_flag(IxgbeRegs::Hlreg0, IXGBE_HLREG0_LPBK);
    }

    // section 4.6.8
    /// Initializes the tx queues of this device.
    fn init_tx(&mut self) {
        // crc offload and small packet padding
        self.write_flag(IxgbeRegs::Hlreg0, IXGBE_HLREG0_TXCRCEN | IXGBE_HLREG0_TXPADEN);

        // section 4.6.11.3.4 - set default buffer size allocations
        self.write_reg_idx(IxgbeArrayRegs::Txpbsize, 0, IXGBE_TXPBSIZE_40KB);
        for i in 1..8 {
            self.write_reg_idx(IxgbeArrayRegs::Txpbsize, i, 0);
        }

        self.write_reg_idx(IxgbeArrayRegs::TxpbThresh, 0, 0xA0);

        for i in 1..8 {
            self.write_reg_idx(IxgbeArrayRegs::TxpbThresh, i, 0);
        }

        // required when not using DCB/VTd
        self.write_reg(IxgbeRegs::Dtxmxszrq, 0xffff);
        self.clear_flag(IxgbeRegs::Rttdcs, IXGBE_RTTDCS_ARBDIS);

        // configure a single transmit queue/ring
        let i: u64 = 0;

        // section 7.1.9 - setup descriptor ring

        self.device.init_tx();

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

        self.write_reg_idx(IxgbeArrayRegs::Txdctl, i, txdctl);

        // final step: enable DMA
        self.write_reg(IxgbeRegs::Dmatxctl, IXGBE_DMATXCTL_TE);
    }

    /// Returns the link speed of this device.
    fn get_link_speed(&self) -> u16 {
        let speed = self.read_reg(IxgbeRegs::Links);
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
        // enable queue and wait if necessary
        self.write_flag_idx(IxgbeArrayRegs::Rxdctl, u64::from(queue_id),
                                        IXGBE_RXDCTL_ENABLE);
        self.wait_write_reg_idx(IxgbeArrayRegs::Rxdctl, u64::from(queue_id),
                                        IXGBE_RXDCTL_ENABLE);

        // rx queue starts out full
        self.device.start_rx_queue(queue_id);
    }

    /// Enables the tx queues.
    ///
    /// # Panics
    /// Panics if length of `self.transmit_ring` is not a power of 2.
    fn start_tx_queue(&mut self, queue_id: u16) {
        self.device.start_tx_queue(queue_id);

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
            count += 1;
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
            speed = self.get_link_speed();
        }
        println!("   - link speed is {} Mbit/s", self.get_link_speed());
    }

    pub fn dump_stats(&self) {
        println!("Ixgbe statistics:");
        let mut string = format!("Stats regs:\n\tGPRC {:08X} GPTC {:08X}\n \
                                 \tGORCL {:08X} GORCH {:08X}\n \
                                 \tGOTCL {:08X} GOTCH {:08X}\n \
                                 \tTXDGPC {:08X} TXDGBCH {:08X} TXDGBCL {:08X} QPTC(0) {:08X}\n \
                                 \t MPTC {:08X} BPTC {:08X}\n",
                                self.read_reg(IxgbeRegs::Gprc) as u32,
                                self.read_reg(IxgbeRegs::Gptc) as u32,
                                self.read_reg(IxgbeRegs::Gorcl) as u32,
                                self.read_reg(IxgbeRegs::Gorch) as u32,
                                self.read_reg(IxgbeRegs::Gotcl) as u32,
                                self.read_reg(IxgbeRegs::Gotch) as u32,
                                self.read_reg(IxgbeRegs::Txdgpc) as u32,
                                self.read_reg(IxgbeRegs::Txdgbch) as u32,
                                self.read_reg(IxgbeRegs::Txdgbcl) as u32,
                                self.read_reg_idx(IxgbeArrayRegs::Qptc, 0) as u32,
                                self.read_reg(IxgbeRegs::Mptc) as u32,
                                self.read_reg(IxgbeRegs::Bptc) as u32,
                                );

        string.push_str(&format!("CRCERRS {:08X} ILLERRC {:08X} ERRBC {:08X}\n \
                                    \tMLFC {:08X} MRFC {:08X} RXMPC[0] {:08X}\n \
                                    \tRLEC {:08X} LXONRXCNT {:08X} LXONRXCNT {:08X}\n \
                                    \tRXDGPC {:08X} RXDGBCL {:08X} RXDGBCH {:08X}\n \
                                    \tRUC {:08X} RFC {:08X} ROC {:08X}\n \
                                    \tRJC {:08X} BPRC {:08X} MPRC {:08X}\n",
                                 self.read_reg(IxgbeRegs::Crcerrs) as u32,
                                 self.read_reg(IxgbeRegs::Illerrc) as u32,
                                 self.read_reg(IxgbeRegs::Errbc) as u32,
                                 self.read_reg(IxgbeRegs::Mlfc) as u32,
                                 self.read_reg(IxgbeRegs::Mrfc) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rxmpc, 0) as u32,
                                 self.read_reg(IxgbeRegs::Rlec) as u32,
                                 self.read_reg(IxgbeRegs::Lxonrxcnt) as u32,
                                 self.read_reg(IxgbeRegs::Lxoffrxcnt) as u32,
                                 self.read_reg(IxgbeRegs::Rxdgpc) as u32,
                                 self.read_reg(IxgbeRegs::Rxdgbch) as u32,
                                 self.read_reg(IxgbeRegs::Rxdgbcl) as u32,
                                 self.read_reg(IxgbeRegs::Ruc) as u32,
                                 self.read_reg(IxgbeRegs::Rfc) as u32,
                                 self.read_reg(IxgbeRegs::Roc) as u32,
                                 self.read_reg(IxgbeRegs::Rjc) as u32,
                                 self.read_reg(IxgbeRegs::Bprc) as u32,
                                 self.read_reg(IxgbeRegs::Mprc) as u32,
                                 ));
        print!("{}", string);
    }

    pub fn dump_all_regs(&self) {
        let mut string = format!("Interrupt regs:\n\tEICR: {:08X} EIMS: {:08X} EIMC: {:08X}\n\tEITR {:08X} GPIE {:08X}\n\tIVAR(0) {:08X}\n",
                    self.read_reg(IxgbeRegs::Eicr) as u32,
                    self.read_reg(IxgbeRegs::Eims) as u32,
                    self.read_reg(IxgbeRegs::Eimc) as u32,
                    self.read_reg_idx(IxgbeArrayRegs::Eitr, 0) as u32,
                    self.read_reg(IxgbeRegs::Gpie) as u32,
                    self.read_reg_idx(IxgbeArrayRegs::Ivar, 0) as u32,
                    );

        string.push_str(&format!("Control regs:\n\tCTRL {:08X} CTRL_EXT {:08X}\n",
                                 self.read_reg(IxgbeRegs::Ctrl) as u32,
                                 self.read_reg(IxgbeRegs::Ctrlext) as u32,
                                 ));

        string.push_str(&format!("EEPROM regs:\n\tEEC_ARD {:08X}\n",
                                 self.read_reg(IxgbeRegs::Eec) as u32));

        string.push_str(&format!("AUTOC {:08X}\n",
                                 self.read_reg(IxgbeRegs::Autoc) as u32));

        string.push_str(&format!("Receive regs:\n\tRDRXCTRL {:08X} RXCTRL {:08X} RXPBSIZE(0): {:08X}\n\tHLREG0 {:08X} FCTRL {:08X}\n\tSRRCTL(0) {:08X} RDBAL(0) {:08X} RDBAH(0) {:08X} RDLEN(0) {:08X}\nRDH(0) {:08X} RDT(0) {:08X}\n",
                                 self.read_reg(IxgbeRegs::Rdrxctl) as u32,
                                 self.read_reg(IxgbeRegs::Rxctrl) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rxpbsize, 0) as u32,
                                 self.read_reg(IxgbeRegs::Hlreg0) as u32,
                                 self.read_reg(IxgbeRegs::Fctrl) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Srrctl, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rdbal, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rdbah, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rdlen, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rdh, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Rdt, 0) as u32,
                                 ));

        string.push_str(&format!("Transmit regs:\n\tTXDCTL(0) {:08X} TXPBSIZE(0): {:08X}\n\tDTXMSSZRQ {:08X} RTTDCS {:08X}\n\tDMATXCTL: {:08X} TDBAL(0) {:08X} TDBAH(0) {:08X} TDLEN(0) {:08X}\n\tTDH(0) {:08X} TDT(0) {:08X}\n",
                                 self.read_reg_idx(IxgbeArrayRegs::Txdctl, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Txpbsize, 0) as u32,
                                 self.read_reg(IxgbeRegs::Dtxmxszrq) as u32,
                                 self.read_reg(IxgbeRegs::Rttdcs) as u32,
                                 self.read_reg(IxgbeRegs::Dmatxctl) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Tdbal, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Tdbah, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Tdlen, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Tdh, 0) as u32,
                                 self.read_reg_idx(IxgbeArrayRegs::Tdt, 0) as u32,
                                 ));
        string.push_str(&format!("Stats regs:\n\tGPRC {:08X} GPTC {:08X}\n\tGORCL {:08X} GORCH {:08X}\n\tGOTCL {:08X} GOTCH {:08X}\n\tTXDGPC {:08X} TXDGBCH {:08X} TXDGBCL {:08X} QPTC(0) {:08X}\n",
                                self.read_reg(IxgbeRegs::Gprc) as u32,
                                self.read_reg(IxgbeRegs::Gptc) as u32,
                                self.read_reg(IxgbeRegs::Gorcl) as u32,
                                self.read_reg(IxgbeRegs::Gorch) as u32,
                                self.read_reg(IxgbeRegs::Gotcl) as u32,
                                self.read_reg(IxgbeRegs::Gotch) as u32,
                                self.read_reg(IxgbeRegs::Txdgpc) as u32,
                                self.read_reg(IxgbeRegs::Txdgbch) as u32,
                                self.read_reg(IxgbeRegs::Txdgbcl) as u32,
                                self.read_reg_idx(IxgbeArrayRegs::Qptc, 0) as u32,
                                ));
        print!("{}", string);
        self.dump_stats();
    }
}
