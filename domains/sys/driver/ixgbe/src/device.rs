#![no_std]




use crate::ixgbe_desc::*;
use crate::Result;
use ixgbe_device::{IxgbeRegs, IxgbeNoDmaArrayRegs};
use ixgbe_device::IxgbeDevice;
use console::{println, print};

use libtime::sys_ns_loopsleep;
use alloc::format;

use crate::PciBarAddr;
use crate::NetworkStats;

const ONE_MS_IN_NS: u64 = 1_000_000 * 1;
const PACKET_SIZE: usize = 60;

pub struct Intel8259x {
    pub device: IxgbeDevice,
}

impl Intel8259x {
    /// Returns an initialized `Intel8259x` on success.
    pub fn new(bar: PciBarAddr) -> Result<Self> {
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
        self.device.read_reg(register)
    }

    fn read_reg_idx(&self, register: IxgbeNoDmaArrayRegs, idx: u64) -> u64 {
        self.device.nd_regs.read_reg_idx(register, idx)
    }

    fn write_reg(&self, register: IxgbeRegs, value: u64) {
        self.device.write_reg(register, value);
    }

    fn write_reg_idx(&self, register: IxgbeNoDmaArrayRegs, idx: u64, value: u64) {
        self.device.nd_regs.write_reg_idx(register, idx, value);
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

    fn write_flag(&self, register: IxgbeRegs, flags: u64) {
        self.write_reg(register, self.read_reg(register) | flags);
    }

    fn clear_flag(&self, register: IxgbeRegs, flags: u64) {
        self.write_reg(register, self.read_reg(register) & !flags);
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
        let ctrl_ext = self.read_reg(IxgbeRegs::CTRL_EXT);
        if (ctrl_ext & IXGBE_CTRL_EXT_NS_DIS) == 0 {
            self.write_reg(IxgbeRegs::CTRL_EXT, ctrl_ext | IXGBE_CTRL_EXT_NS_DIS);
        }
        self.write_reg(IxgbeRegs::CTRL_EXT, IXGBE_CTRL_EXT_DRV_LOAD);

        self.write_reg(IxgbeRegs::CTRL_EXT, IXGBE_CTRL_EXT_DRV_LOAD);

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
        println!("Sleep for 15 seconds");
        sys_ns_loopsleep(ONE_MS_IN_NS * 1000 * 3);
        println!("Resuming sleep");
    }

    /// Returns the mac address of this device.
    pub fn get_mac_addr(&self) -> [u8; 6] {
        let low = self.read_reg_idx(IxgbeNoDmaArrayRegs::Ral, 0);
        let high = self.read_reg_idx(IxgbeNoDmaArrayRegs::Rah, 0);

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


        self.write_reg_idx(IxgbeNoDmaArrayRegs::Ral, 0, low as u64);
        self.write_reg_idx(IxgbeNoDmaArrayRegs::Rah, 0, high as u64);
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

        // enable CRC offloading
        self.write_flag(IxgbeRegs::HLREG0, IXGBE_HLREG0_RXCRCSTRP);
        self.write_flag(IxgbeRegs::RDRXCTL, IXGBE_RDRXCTL_CRCSTRIP);

        // accept broadcast packets
        self.write_flag(IxgbeRegs::FCTRL, IXGBE_FCTRL_BAM);

        // configure a single receive queue/ring
        let _i: u64 = 0;

        // TODO: Manipulation of rx queue. Move this to trusted part
        self.device.init_rx();

        // last sentence of section 4.6.7 - set some magic bits
        self.write_flag(IxgbeRegs::CTRL_EXT, IXGBE_CTRL_EXT_NS_DIS);

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

        // required when not using DCB/VTd
        self.write_reg(IxgbeRegs::DTXMXSZRQ, 0xffff);
        self.clear_flag(IxgbeRegs::RTTDCS, IXGBE_RTTDCS_ARBDIS);

        // configure a single transmit queue/ring
        let _i: u64 = 0;

        // section 7.1.9 - setup descriptor ring

        self.device.init_tx();

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
        self.device.start_rx_queue(queue_id);
    }

    /// Enables the tx queues.
    ///
    /// # Panics
    /// Panics if length of `self.transmit_ring` is not a power of 2.
    fn start_tx_queue(&mut self, queue_id: u16) {
        self.device.start_tx_queue(queue_id);
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
            count += 1;
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
            speed = self.get_link_speed();
        }
        println!("   - link speed is {} Mbit/s", self.get_link_speed());
    }

    pub fn dump_rx_descs(&mut self) {
        self.device.dump_rx_descs();
    }

    pub fn dump_tx_descs(&mut self) {
        self.device.dump_tx_descs();
    }

    pub fn dump_stats(&self) {
        println!("Ixgbe statistics:");
        let mut string = format!("Stats regs:\n\tGPRC {:08X} GPTC {:08X}\n \
                                 \tGORCL {:08X} GORCH {:08X}\n \
                                 \tGOTCL {:08X} GOTCH {:08X}\n \
                                 \tTXDGPC {:08X} TXDGBCH {:08X} TXDGBCL {:08X} QPTC(0) {:08X}\n \
                                 \t MPTC {:08X} BPTC {:08X}\n",
                                self.read_reg(IxgbeRegs::GPRC) as u32,
                                self.read_reg(IxgbeRegs::GPTC) as u32,
                                self.read_reg(IxgbeRegs::GORCL) as u32,
                                self.read_reg(IxgbeRegs::GORCH) as u32,
                                self.read_reg(IxgbeRegs::GOTCL) as u32,
                                self.read_reg(IxgbeRegs::GOTCH) as u32,
                                self.read_reg(IxgbeRegs::TXDGPC) as u32,
                                self.read_reg(IxgbeRegs::TXDGBCH) as u32,
                                self.read_reg(IxgbeRegs::TXDGBCL) as u32,
                                self.read_reg_idx(IxgbeNoDmaArrayRegs::Qptc, 0) as u32,
                                self.read_reg(IxgbeRegs::MPTC) as u32,
                                self.read_reg(IxgbeRegs::BPTC) as u32,
                                );

        string.push_str(&format!("CRCERRS {:08X} ILLERRC {:08X} ERRBC {:08X}\n \
                                    \tMLFC {:08X} MRFC {:08X} RXMPC[0] {:08X}\n \
                                    \tRLEC {:08X} LXONRXCNT {:08X} LXONRXCNT {:08X}\n \
                                    \tRXDGPC {:08X} RXDGBCL {:08X} RXDGBCH {:08X}\n \
                                    \tRUC {:08X} RFC {:08X} ROC {:08X}\n \
                                    \tRJC {:08X} BPRC {:08X} MPRC {:08X}\n",
                                 self.read_reg(IxgbeRegs::CRCERRS) as u32,
                                 self.read_reg(IxgbeRegs::ILLERRC) as u32,
                                 self.read_reg(IxgbeRegs::ERRBC) as u32,
                                 self.read_reg(IxgbeRegs::MLFC) as u32,
                                 self.read_reg(IxgbeRegs::MRFC) as u32,
                                 self.read_reg_idx(IxgbeNoDmaArrayRegs::Rxmpc, 0) as u32,
                                 self.read_reg(IxgbeRegs::RLEC) as u32,
                                 self.read_reg(IxgbeRegs::LXONRXCNT) as u32,
                                 self.read_reg(IxgbeRegs::LXOFFRXCNT) as u32,
                                 self.read_reg(IxgbeRegs::RXDGPC) as u32,
                                 self.read_reg(IxgbeRegs::RXDGBCH) as u32,
                                 self.read_reg(IxgbeRegs::RXDGBCL) as u32,
                                 self.read_reg(IxgbeRegs::RUC) as u32,
                                 self.read_reg(IxgbeRegs::RFC) as u32,
                                 self.read_reg(IxgbeRegs::ROC) as u32,
                                 self.read_reg(IxgbeRegs::RJC) as u32,
                                 self.read_reg(IxgbeRegs::BPRC) as u32,
                                 self.read_reg(IxgbeRegs::MPRC) as u32,
                                 ));
        print!("{}", string);
    }

    pub fn dump_all_regs(&self) {
        let mut string = format!("Interrupt regs:\n\tEICR: {:08X} EIMS: {:08X} EIMC: {:08X}\n\tGPIE {:08X}\n",
                    self.read_reg(IxgbeRegs::EICR) as u32,
                    self.read_reg(IxgbeRegs::EIMS) as u32,
                    self.read_reg(IxgbeRegs::EIMC) as u32,
                    self.read_reg(IxgbeRegs::GPIE) as u32,
                    );

        string.push_str(&format!("Control regs:\n\tCTRL {:08X} CTRL_EXT {:08X}\n",
                                 self.read_reg(IxgbeRegs::CTRL) as u32,
                                 self.read_reg(IxgbeRegs::CTRL_EXT) as u32,
                                 ));

        string.push_str(&format!("EEPROM regs:\n\tEEC_ARD {:08X}\n",
                                 self.read_reg(IxgbeRegs::EEC) as u32));

        string.push_str(&format!("AUTOC {:08X}\n",
                                 self.read_reg(IxgbeRegs::AUTOC) as u32));

        string.push_str(&format!("Receive regs:\n\tRDRXCTRL {:08X} RXCTRL {:08X}\n\tHLREG0 {:08X} FCTRL {:08X}\n",
                                 self.read_reg(IxgbeRegs::RDRXCTL) as u32,
                                 self.read_reg(IxgbeRegs::RXCTRL) as u32,
                                 self.read_reg(IxgbeRegs::HLREG0) as u32,
                                 self.read_reg(IxgbeRegs::FCTRL) as u32,
                                 ));

        string.push_str(&format!("Transmit regs:\n\tDTXMSSZRQ {:08X} RTTDCS {:08X} DMATXCTL: {:08X}\n",
                                 self.read_reg(IxgbeRegs::DTXMXSZRQ) as u32,
                                 self.read_reg(IxgbeRegs::RTTDCS) as u32,
                                 self.read_reg(IxgbeRegs::DMATXCTL) as u32,
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
                                self.read_reg_idx(IxgbeNoDmaArrayRegs::Qptc, 0) as u32,
                                ));
        print!("{}", string);

        self.device.dump_dma_regs();

        self.dump_stats();
    }

    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            tx_count: self.read_reg(IxgbeRegs::GPTC),
            rx_count: self.read_reg(IxgbeRegs::GPRC),
            tx_dma_ok: self.read_reg(IxgbeRegs::TXDGPC),
            rx_dma_ok: self.read_reg(IxgbeRegs::RXDGPC),
            rx_missed: self.read_reg_idx(IxgbeNoDmaArrayRegs::Rxmpc, 0),
            rx_crc_err: self.read_reg(IxgbeRegs::CRCERRS),
        }
    }
}
