#![no_std]

mod ixgbe_regs;
extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::boxed::Box;
use array_init::array_init;
use console::{println, print};
use core::{mem, ptr};
use ixgbe_regs::IxgbeDmaArrayRegs;
use libdma::Dma;
use libdma::ixgbe::{allocate_dma, ixgbe_adv_rx_desc, ixgbe_adv_tx_desc};
use platform::PciBarAddr;
use ixgbe_regs::{IxgbeDmaRegs, IxgbeNonDmaRegs};
use libtime::sys_ns_loopsleep;
use alloc::format;
use rref::{RRef, RRefDeque};
pub use ixgbe_regs::{IxgbeRegs, IxgbeNoDmaArrayRegs};

const TX_CLEAN_BATCH: usize = 32;

const IXGBE_SRRCTL_DESCTYPE_MASK: u64       = 0x0E000000;
const IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF: u64 = 0x02000000;
const IXGBE_SRRCTL_DROP_EN: u64             = 0x10000000;

const IXGBE_RXD_STAT_DD: u32                = 0x01; /* Descriptor Done */
const IXGBE_RXD_STAT_EOP: u32               = 0x02; /* End of Packet */
const IXGBE_RXDADV_STAT_DD: u32             = IXGBE_RXD_STAT_DD; /* Done */
const IXGBE_RXDADV_STAT_EOP: u32            = IXGBE_RXD_STAT_EOP; /* End of Packet */

const IXGBE_ADVTXD_PAYLEN_SHIFT: u32        = 14; /* Adv desc PAYLEN shift */
const IXGBE_TXD_CMD_EOP: u32                = 0x01000000; /* End of Packet */
const IXGBE_ADVTXD_DCMD_EOP: u32            = IXGBE_TXD_CMD_EOP; /* End of Packet */
const IXGBE_TXD_CMD_RS: u32                 = 0x08000000; /* Report Status */
const IXGBE_ADVTXD_DCMD_RS: u32             = IXGBE_TXD_CMD_RS; /* Report Status */
const IXGBE_TXD_CMD_IFCS: u32               = 0x02000000; /* Insert FCS (Ethernet CRC) */
const IXGBE_ADVTXD_DCMD_IFCS: u32           = IXGBE_TXD_CMD_IFCS; /* Insert FCS */
const IXGBE_TXD_CMD_DEXT: u32               = 0x20000000; /* Desc extension (0 = legacy) */
const IXGBE_ADVTXD_DTYP_DATA: u32           = 0x00300000; /* Adv Data Descriptor */
const IXGBE_ADVTXD_DCMD_DEXT: u32           = IXGBE_TXD_CMD_DEXT; /* Desc ext 1=Adv */
const IXGBE_TXD_STAT_DD: u32                = 0x00000001; /* Descriptor Done */
const IXGBE_ADVTXD_STAT_DD: u32             = IXGBE_TXD_STAT_DD; /* Descriptor Done */

const IXGBE_TXPBSIZE_40KB: u64              = 0x0000A000; /* 40KB Packet Buffer */
const IXGBE_RXPBSIZE_128KB: u64             = 0x00020000; /* 128KB Packet Buffer */
const IXGBE_RXDCTL_ENABLE: u64              = 0x02000000; /* Ena specific Rx Queue */
const IXGBE_TXDCTL_ENABLE: u64              = 0x02000000; /* Ena specific Tx Queue */

const ONE_MS_IN_NS: u64 = 1_000_000 * 1;

const NUM_TX_DESCS: usize = 512;
const NUM_RX_DESCS: usize = 512;

pub struct IxgbeDevice {
    //pub bar: Box<dyn IxgbeBarRegion>,
    bar: PciBarAddr,
    transmit_buffers: [Option<Vec<u8>>; NUM_TX_DESCS],
    transmit_rrefs: [Option<RRef<[u8; 1512]>>; NUM_TX_DESCS],
    transmit_ring: Dma<[ixgbe_adv_tx_desc; NUM_TX_DESCS]>,
    receive_buffers: [Option<Vec<u8>>; NUM_RX_DESCS],
    receive_rrefs: [Option<RRef<[u8; 1512]>>; NUM_TX_DESCS],
    receive_ring: Dma<[ixgbe_adv_rx_desc; NUM_RX_DESCS]>,
    tx_slot: [bool; NUM_TX_DESCS],
    rx_slot: [bool; NUM_RX_DESCS],
    transmit_index: usize,
    transmit_clean_index: usize,
    rx_clean_index: usize,
    tx_clean_index: usize,
    receive_index: usize,
    regs: IxgbeDmaRegs,
    pub nd_regs: IxgbeNonDmaRegs,
    dump: bool,
    rx_dump: bool,
}

fn wrap_ring(index: usize, ring_size: usize) -> usize {
    (index + 1) & (ring_size - 1)
}

impl IxgbeDevice {
    pub fn new(bar: PciBarAddr) -> IxgbeDevice {
        IxgbeDevice {
            bar,
            transmit_buffers: array_init::array_init(|_| None),
            transmit_rrefs: array_init::array_init(|_| None),
            receive_rrefs: array_init::array_init(|_| None),
            receive_buffers: array_init::array_init(|_| None),
            transmit_index: 0,
            transmit_clean_index: 0,
            rx_clean_index: 0,
            tx_clean_index: 0,
            receive_index: 0,
            tx_slot: [false; NUM_TX_DESCS],
            rx_slot: [false; NUM_RX_DESCS],
            receive_ring: allocate_dma().unwrap(),
            transmit_ring: allocate_dma().unwrap(),
            regs: unsafe { IxgbeDmaRegs::new(bar) },
            nd_regs: unsafe { IxgbeNonDmaRegs::new(bar) },
            dump: false,
            rx_dump: false,
        }
    }

    pub fn read_reg(&self, reg: IxgbeRegs) -> u64 {
        unsafe {
            ptr::read_volatile((self.bar.get_base() as u64 + reg as u64) as *const u64) & 0xFFFF_FFFF as u64
        }
    }

    pub fn write_reg(&self, reg: IxgbeRegs, val: u64) {
        unsafe {
            println!("writing to {:x}", self.bar.get_base() as u64 + reg as u64);
            ptr::write_volatile((self.bar.get_base() as u64 + reg as u64) as *mut u32, val as u32);
        }
    }

    fn read_qreg_idx(&self, reg: IxgbeDmaArrayRegs, idx: u64) -> u64 {
        self.regs.read_reg_idx(reg, idx)
    }

    fn write_qreg_idx(&self, reg: IxgbeDmaArrayRegs, idx: u64, val: u64) {
        self.regs.write_reg_idx(reg, idx, val);
    }

    fn write_qflag_idx(&self, register: IxgbeDmaArrayRegs, idx: u64, flags: u64) {
        self.write_qreg_idx(register, idx, self.read_qreg_idx(register, idx) | flags);
    }

    fn wait_write_qreg_idx(&self, register: IxgbeDmaArrayRegs, idx: u64, value: u64) {
        loop {
            let current = self.read_qreg_idx(register, idx);
            if (current & value) == value {
                break;
            }
            sys_ns_loopsleep(ONE_MS_IN_NS * 100);
        }
    }

    fn clear_qflag_idx(&self, register: IxgbeDmaArrayRegs, idx: u64, flags: u64) {
        self.write_qreg_idx(register, idx, self.read_qreg_idx(register, idx) & !flags);
    }


    pub fn start_rx_queue(&self, queue_id: u16) {
        // enable queue and wait if necessary
        self.write_qflag_idx(IxgbeDmaArrayRegs::Rxdctl, u64::from(queue_id),
                                        IXGBE_RXDCTL_ENABLE);
        self.wait_write_qreg_idx(IxgbeDmaArrayRegs::Rxdctl, u64::from(queue_id),
                                        IXGBE_RXDCTL_ENABLE);


        // rx queue starts out full
        self.write_qreg_idx(IxgbeDmaArrayRegs::Rdh, u64::from(queue_id), 0);
        self.write_qreg_idx(IxgbeDmaArrayRegs::Rdt, u64::from(queue_id), 0);
    }

    pub fn start_tx_queue(&self, queue_id: u16) {
        self.write_qreg_idx(IxgbeDmaArrayRegs::Tdh, u64::from(queue_id), 0);
        self.write_qreg_idx(IxgbeDmaArrayRegs::Tdt, u64::from(queue_id), 0);

        // enable queue and wait if necessary
        self.write_qflag_idx(IxgbeDmaArrayRegs::Txdctl, u64::from(queue_id),
                                            IXGBE_TXDCTL_ENABLE);
        self.wait_write_qreg_idx(IxgbeDmaArrayRegs::Txdctl, u64::from(queue_id),
                                            IXGBE_TXDCTL_ENABLE);
    }

    pub fn init_rx(&self) {
        let i: u64 = 0;

        // probably a broken feature, this flag is initialized with 1 but has to be set to 0
        self.clear_qflag_idx(IxgbeDmaArrayRegs::DcaRxctrl, i, 1 << 12);


        // section 4.6.11.3.4 - allocate all queues and traffic to PB0
        self.write_qreg_idx(IxgbeDmaArrayRegs::Rxpbsize, 0, IXGBE_RXPBSIZE_128KB);

        for i in 1..8 {
            self.write_qreg_idx(IxgbeDmaArrayRegs::Rxpbsize, i, 0);
        }


        self.write_qreg_idx(
            IxgbeDmaArrayRegs::Srrctl, i,
            (self.read_qreg_idx(IxgbeDmaArrayRegs::Srrctl, i) & !IXGBE_SRRCTL_DESCTYPE_MASK)
                | IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF,
        );

        // let nic drop packets if no rx descriptor is available instead of buffering them
        self.write_qreg_idx(
            IxgbeDmaArrayRegs::Srrctl, i,
            self.read_qreg_idx(IxgbeDmaArrayRegs::Srrctl, i) | IXGBE_SRRCTL_DROP_EN,
        );

        self.write_qreg_idx(IxgbeDmaArrayRegs::Rdbal, i, (self.receive_ring.physical() & 0xffff_ffff) as u64);

        self.write_qreg_idx(IxgbeDmaArrayRegs::Rdbah, i, (self.receive_ring.physical() >> 32) as u64);

        println!("rx ring {} phys addr: {:#x}", i, self.receive_ring.physical());

        self.write_qreg_idx(
            IxgbeDmaArrayRegs::Rdlen, i,
            (self.receive_ring.len() * mem::size_of::<ixgbe_adv_rx_desc>()) as u64,
        );
    }

    pub fn init_tx(&self) {
        let i: u64 = 0;

        // section 4.6.11.3.4 - set default buffer size allocations
        self.write_qreg_idx(IxgbeDmaArrayRegs::Txpbsize, 0, IXGBE_TXPBSIZE_40KB);
        for i in 1..8 {
            self.write_qreg_idx(IxgbeDmaArrayRegs::Txpbsize, i, 0);
        }

        self.write_qreg_idx(IxgbeDmaArrayRegs::TxpbThresh, 0, 0xA0);

        for i in 1..8 {
            self.write_qreg_idx(IxgbeDmaArrayRegs::TxpbThresh, i, 0);
        }

        self.write_qreg_idx(IxgbeDmaArrayRegs::Tdbal, i,
                                self.transmit_ring.physical() as u64);
        self.write_qreg_idx(IxgbeDmaArrayRegs::Tdbah, i,
                               (self.transmit_ring.physical() >> 32) as u64);

        println!("tx ring {} phys addr: {:#x}", i, self.transmit_ring.physical());
        self.write_qreg_idx(IxgbeDmaArrayRegs::Tdlen, i,
            (self.transmit_ring.len() * mem::size_of::<ixgbe_adv_tx_desc>()) as u64
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

        self.write_qreg_idx(IxgbeDmaArrayRegs::Txdctl, i, txdctl);
    }

    fn clean_tx_queue(&mut self) -> usize {
        let mut clean_index = self.transmit_clean_index;
        let cur_index = self.transmit_index;

        loop {
            let mut cleanable = cur_index as i32 - clean_index as i32;
            let num_descriptors = self.transmit_ring.len();

            if cleanable < 0 {
                cleanable += num_descriptors as i32;
            }

            if cleanable < TX_CLEAN_BATCH as i32 {
                break;
            }

            let mut cleanup_to = clean_index + TX_CLEAN_BATCH - 1;

            if cleanup_to >= num_descriptors {
                cleanup_to -= num_descriptors;
            }

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

    pub fn submit(&mut self, packets: &mut VecDeque<Vec<u8>>) -> usize {
        let mut sent = 0;
        let mut cur_index = self.transmit_index;
        let clean_index = self.clean_tx_queue();
        let num_descriptors = self.transmit_ring.len();

        while let Some(packet) = packets.pop_front() {
            let next_index = wrap_ring(cur_index, num_descriptors);

            if clean_index == next_index {
                // tx queue of device is full, push packet back onto the
                // queue of to-be-sent packets
                packets.push_front(packet);
                break;
            }

            self.transmit_index = wrap_ring(self.transmit_index, num_descriptors);

            let pkt_len = packet.len();

            unsafe {
                self.transmit_ring[cur_index].read.buffer_addr = packet.as_ptr() as u64;

                core::ptr::write_volatile(
                        &(*self.transmit_ring.as_ptr().add(cur_index)).read.buffer_addr as *const u64 as *mut u64,
                        packet.as_ptr() as u64);

                self.transmit_buffers[cur_index] = Some(packet);
                self.tx_slot[cur_index] = true;

                core::ptr::write_volatile(
                        &(*self.transmit_ring.as_ptr().add(cur_index)).read.cmd_type_len as *const u32 as *mut u32,
                        IXGBE_ADVTXD_DCMD_EOP
                                | IXGBE_ADVTXD_DCMD_RS
                                | IXGBE_ADVTXD_DCMD_IFCS
                                | IXGBE_ADVTXD_DCMD_DEXT
                                | IXGBE_ADVTXD_DTYP_DATA
                                | pkt_len as u32,
                );

                core::ptr::write_volatile(
                        &(*self.transmit_ring.as_ptr().add(cur_index)).read.olinfo_status as *const u32 as *mut u32,
                        (pkt_len as u32) << IXGBE_ADVTXD_PAYLEN_SHIFT,
                );
            }

            cur_index = next_index;
            sent += 1;
        }

        if sent > 0 {
            //self.bar.write_reg_tdt(0, self.transmit_index as u64);
            self.write_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0, self.transmit_index as u64);
        }

        sent
    }

    pub fn submit_and_poll_rref(&mut self, mut packets: RRefDeque<[u8; 1512], 32>, mut collect: RRefDeque<[u8; 1512], 32>, tx: bool, debug: bool) ->
            (usize, RRefDeque<[u8; 1512], 32>, RRefDeque<[u8; 1512], 32>)
    {
        if tx {
            self.tx_submit_and_poll_rref(packets, collect, debug)
        } else {
            self.rx_submit_and_poll_rref(packets, collect, debug)
        }
    }

    pub fn submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool, debug: bool) -> usize {
        if tx {
            self.tx_submit_and_poll(packets, reap_queue, debug)
        } else {
            self.rx_submit_and_poll(packets, reap_queue, debug)
        }
    }

    pub fn tx_poll_rref(&mut self,
                        mut reap_queue: RRefDeque<[u8; 1512], 512>) ->
                        (usize, RRefDeque<[u8; 1512], 512>) {

        let num_descriptors = self.transmit_ring.len();
        let mut reaped: usize = 0;
        let mut count: usize = 0;
        let mut tx_clean_index: usize = self.tx_clean_index;

        for tx_index in 0..num_descriptors {
            let status = unsafe {
                core::ptr::read_volatile(&(*self.transmit_ring.as_ptr().add(tx_index)).wb.status
                   as *const u32)
            };

            if (status & IXGBE_ADVTXD_STAT_DD) != 0 {
                if self.tx_slot[tx_index] {
                    count += 1;
                    if let Some(mut pkt) = self.transmit_rrefs[tx_index].take() {
                        if reap_queue.push_back(pkt).is_some() {
                            println!("tx_poll_rref: Pushing to full reap queue");
                        }
                    }
                    self.tx_slot[tx_index] = false;
                    self.transmit_rrefs[tx_index] = None;
                    reaped += 1;
                    tx_clean_index = tx_index;
                }
            }
        }
        println!("Found {} sent DDs", count);
        let head = self.read_qreg_idx(IxgbeDmaArrayRegs::Tdh, 0);
        let tail = self.read_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0);

        print!("Tx ring {:16x} len {} HEAD {} TAIL {}\n", self.transmit_ring.physical(), self.transmit_ring.len(), head, tail);

        if reaped > 0 {
            self.tx_clean_index = self.transmit_index;
        }
        (reaped, reap_queue)
    }

    pub fn rx_poll_rref(&mut self,
                        mut reap_queue: RRefDeque<[u8; 1512], 32>) ->
                        (usize, RRefDeque<[u8; 1512], 32>) {


        let num_descriptors = self.receive_ring.len();
        let mut reaped: usize = 0;
        let mut count: usize = 0;
        let mut rx_clean_index: usize = self.rx_clean_index;

        for rx_index in 0..num_descriptors {
            let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_index) as *mut ixgbe_adv_rx_desc) };

            let status = unsafe {
                    core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32)
            };

            if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                panic!("increase buffer size or decrease MTU")
            }

            if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                if self.rx_slot[rx_index] {
                    count += 1;
                    if let Some(mut pkt) = self.receive_rrefs[rx_index].take() {

                        //println!("{}, buffer {:16x}", rx_index, pkt.as_ptr() as u64);

                        let length = unsafe { core::ptr::read_volatile(
                                &(*desc).wb.upper.length as *const u16) as usize
                        };

                        if reap_queue.push_back(pkt).is_some() {
                            println!("rx_poll_rref: Pushing to full reap queue");
                        }
                    }
                    self.rx_slot[rx_index] = false;
                    self.receive_rrefs[rx_index] = None;
                    reaped += 1;
                    rx_clean_index = rx_index;
                }
            }
        }
        println!("Found {} sent DDs", count);

        let head = self.read_qreg_idx(IxgbeDmaArrayRegs::Rdh, 0);
        let tail = self.read_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0);

        print!("rx_index {} rx_clean_index {}\n", self.receive_index, self.rx_clean_index);
        print!("Rx ring {:16x} len {} HEAD {} TAIL {}\n", self.receive_ring.physical(), self.receive_ring.len(), head, tail);

        if reaped > 0 {
            println!("update clean index to {}", rx_clean_index);
            self.rx_clean_index = self.receive_index;
        }
        (reaped, reap_queue)
    }

    pub fn tx_poll(&mut self,  reap_queue: &mut VecDeque<Vec<u8>>) -> usize {
        let num_descriptors = self.transmit_ring.len();
        let mut reaped: usize = 0;
        let mut count: usize = 0;
        let mut tx_clean_index: usize = self.tx_clean_index;

        for tx_index in 0..num_descriptors {
            let status = unsafe {
                core::ptr::read_volatile(&(*self.transmit_ring.as_ptr().add(tx_index)).wb.status
                   as *const u32)
            };

            if (status & IXGBE_ADVTXD_STAT_DD) != 0 {
                if self.tx_slot[tx_index] {
                    count += 1;
                    if let Some(pkt) = &mut self.transmit_buffers[tx_index] {
                        let mut buf = pkt.as_mut_ptr();
                        let vec = unsafe { Vec::from_raw_parts(buf, pkt.len(), pkt.capacity()) };
                        reap_queue.push_front(vec);
                    }
                    self.tx_slot[tx_index] = false;
                    self.transmit_buffers[tx_index] = None;
                    reaped += 1;
                    tx_clean_index = tx_index;
                }
            }
        }
        println!("Found {} sent DDs", count);
        let head = self.read_qreg_idx(IxgbeDmaArrayRegs::Tdh, 0);
        let tail = self.read_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0);

        print!("Tx ring {:16x} len {} HEAD {} TAIL {}\n", self.transmit_ring.physical(), self.transmit_ring.len(), head, tail);

        if reaped > 0 {
            self.tx_clean_index = self.transmit_index;
        }
        reaped
    }


    pub fn rx_poll(&mut self,  reap_queue: &mut VecDeque<Vec<u8>>) -> usize {
        let num_descriptors = self.receive_ring.len();
        let mut reaped: usize = 0;
        let mut count: usize = 0;
        let mut rx_clean_index: usize = self.rx_clean_index;

        for rx_index in 0..num_descriptors {
            let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_index) as *mut ixgbe_adv_rx_desc) };

            let status = unsafe {
                    core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32)
            };

            if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                panic!("increase buffer size or decrease MTU")
            }

            if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                if self.rx_slot[rx_index] {
                    count += 1;
                    if let Some(pkt) = &mut self.receive_buffers[rx_index] {

                        //println!("{}, buffer {:16x}", rx_index, pkt.as_ptr() as u64);

                        let length = unsafe { core::ptr::read_volatile(
                                &(*desc).wb.upper.length as *const u16) as usize
                        };

                        let mut buf = pkt.as_mut_ptr();
                        let vec = unsafe { Vec::from_raw_parts(buf, length, pkt.capacity()) };
                        reap_queue.push_front(vec);
                    }
                    self.rx_slot[rx_index] = false;
                    self.receive_buffers[rx_index] = None;
                    reaped += 1;
                    rx_clean_index = rx_index;
                }
            }
        }
        println!("Found {} sent DDs", count);

        let head = self.read_qreg_idx(IxgbeDmaArrayRegs::Rdh, 0);
        let tail = self.read_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0);

        print!("rx_index {} rx_clean_index {}\n", self.receive_index, self.rx_clean_index);
        print!("Rx ring {:16x} len {} HEAD {} TAIL {}\n", self.receive_ring.physical(), self.receive_ring.len(), head, tail);

        if reaped > 0 {
            println!("update clean index to {}", rx_clean_index);
            self.rx_clean_index = self.receive_index;
        }
        reaped
    }

    fn dump_rx_desc(&mut self) {
        print!("=====================\n");
        print!("Rx descriptors\n");
        print!("=====================\n");

        let head = self.read_qreg_idx(IxgbeDmaArrayRegs::Rdh, 0);
        let tail = self.read_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0);

        print!("Rx ring {:16x} len {} HEAD {} TAIL {}\n", self.receive_ring.physical(), self.receive_ring.len(), head, tail);
        print!("rx_index: {} rx_clean_index {}\n", self.receive_index, self.rx_clean_index);

        let mut str = format!("[Idx]  [buffer]   [slot]   [status]\n");
        for i in 0..self.receive_ring.len() {
            let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(i) as *mut ixgbe_adv_rx_desc) };

            let status = unsafe {
                core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32) };

            if i == head as usize {
                str.push_str(&format!("H=>"));
            } 
            
            if i == tail as usize {
                str.push_str(&format!("T=>"));
            }

            let mut buffer: u64 = 0;
            if let Some(pkt) = &self.receive_buffers[i] {
                buffer = pkt.as_ptr() as u64;
            }
            str.push_str(&format!("[{}] buffer: {:16x} rx_slot {} status {:x}\n", i, buffer, self.rx_slot[i], status));
        } 
        print!("{}", str);
        self.rx_dump = true;
    }

    fn dump_tx_desc(&mut self) {
        print!("=====================\n");
        print!("Tx descriptors\n");
        print!("=====================\n");

        let head = self.read_qreg_idx(IxgbeDmaArrayRegs::Tdh, 0);
        let tail = self.read_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0);

        print!("Tx ring {:16x} len {} HEAD {} TAIL {}\n", self.transmit_ring.physical(), self.transmit_ring.len(), head, tail);

        let mut str = format!("[Idx]  [buffer]   [slot]   [status]\n");
        for i in 0..self.transmit_ring.len() {
            let mut desc = unsafe { &mut*(self.transmit_ring.as_ptr().add(i) as *mut ixgbe_adv_tx_desc) };

            let status = unsafe {
                core::ptr::read_volatile(&mut (*desc).wb.status as *mut u32) };

            if i == head as usize {
                str.push_str(&format!("H=>"));
            } 
            
            if i == tail as usize {
                str.push_str(&format!("T=>"));
            }

            let mut buffer: u64 = 0;
            if let Some(pkt) = &self.transmit_buffers[i] {
                buffer = pkt.as_ptr() as u64;
            }
            str.push_str(&format!("[{}] buffer: {:16x} tx_slot {} status {:x}\n", i, buffer, self.tx_slot[i], status));
        } 
        print!("{}", str);
        self.dump = true;
    }

    pub fn dump_rx_descs(&mut self) {
        self.dump_rx_desc();
        self.rx_dump = false;
    }

    pub fn dump_tx_descs(&mut self) {
        self.dump_tx_desc();
        self.dump = false;
    }

    #[inline(always)]
    fn tx_submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, debug: bool) -> usize {
        let mut sent = 0;
        let mut tx_index = self.transmit_index;
        let mut tx_clean_index = self.tx_clean_index;
        let mut last_tx_index = self.transmit_index;
        let num_descriptors = self.transmit_ring.len();
        let BATCH_SIZE = 32;


        if packets.len() > 0 {
            if debug {
                println!("tx index {} packets {}", tx_index, packets.len());
            }
            while let Some(packet) = packets.pop_front() {

                //println!("Found packet!");
                let mut desc = unsafe { &mut*(self.transmit_ring.as_ptr().add(tx_index) as *mut ixgbe_adv_tx_desc) };

                let status = unsafe {
                    core::ptr::read_volatile(&mut (*desc).wb.status as *mut u32) };

                unsafe {
                    //println!("pkt_addr {:08X} tx_Buffer {:08X}",
                    //            (*desc).read.pkt_addr as *const u64 as u64,
                    //            self.transmit_buffer[tx_index].physical());
                }

                // DD == 0 on a TX desc leaves us with 2 possibilities
                // 1) The desc is populated (tx_slot[i] = true), the device did not sent it out yet
                // 2) The desc is not populated. In that case, tx_slot[i] = false
                if ((status & IXGBE_RXDADV_STAT_DD) == 0) && self.tx_slot[tx_index] {
                    if debug {
                        println!("No free slot. Fucked");
                        if !self.dump {
                            self.dump_tx_desc();
                        }
                    }
                    packets.push_front(packet);
                    break;
                }

                let pkt_len = packet.len();
                if debug {
                    println!("packet len {}", pkt_len);
                }
                unsafe {
                    if self.tx_slot[tx_index] {
                        if let Some(pkt) = &mut self.transmit_buffers[tx_index] {
                            let mut buf = pkt.as_mut_ptr();
                            let vec = Vec::from_raw_parts(buf, pkt_len, pkt.capacity());
                            if debug {
                                println!("buf {:x} vec_raw_parts {:x}", buf as u64, vec.as_ptr() as u64);
                            }
                            reap_queue.push_front(vec);
                        }

                        tx_clean_index = wrap_ring(tx_clean_index, self.transmit_ring.len());
                    }


                    if debug {
                        println!("programming new buffer! {:x} packet[0] {:x}", packet.as_ptr() as u64, packet[0]);
                    }
                    // switch to a new buffer
                    core::ptr::write_volatile(
                        &(*self.transmit_ring.as_ptr().add(tx_index)).read.buffer_addr as *const u64 as *mut u64,
                        packet.as_ptr() as u64);

                    self.transmit_buffers[tx_index] = Some(packet);
                    self.tx_slot[tx_index] = true;

                    core::ptr::write_volatile(
                            &(*self.transmit_ring.as_ptr().add(tx_index)).read.cmd_type_len as *const u32 as *mut u32,
                            IXGBE_ADVTXD_DCMD_EOP
                                    | IXGBE_ADVTXD_DCMD_RS
                                    | IXGBE_ADVTXD_DCMD_IFCS
                                    | IXGBE_ADVTXD_DCMD_DEXT
                                    | IXGBE_ADVTXD_DTYP_DATA
                                    | pkt_len as u32,
                    );

                    core::ptr::write_volatile(
                            &(*self.transmit_ring.as_ptr().add(tx_index)).read.olinfo_status as *const u32 as *mut u32,
                            (pkt_len as u32) << IXGBE_ADVTXD_PAYLEN_SHIFT,
                    );
                }

                last_tx_index = tx_index;
                tx_index = wrap_ring(tx_index, self.transmit_ring.len());
                sent += 1;
            }
            if reap_queue.len() < BATCH_SIZE {
                let mut reaped = 0;
                let mut count = 0;
                let batch = BATCH_SIZE - reap_queue.len();

                loop {
                    let status = unsafe {
                        core::ptr::read_volatile(&(*self.transmit_ring.as_ptr().add(tx_clean_index)).wb.status
                           as *const u32)
                    };

                    if (status & IXGBE_ADVTXD_STAT_DD) != 0 {
                        if self.tx_slot[tx_clean_index] {
                            if let Some(pkt) = &mut self.transmit_buffers[tx_clean_index] {
                                let mut buf = pkt.as_mut_ptr();
                                let vec = unsafe { Vec::from_raw_parts(buf, pkt.len(), pkt.capacity()) };
                                reap_queue.push_front(vec);
                            }
                            self.tx_slot[tx_clean_index] = false;
                            self.transmit_buffers[tx_clean_index] = None;
                            reaped += 1;
                        }
                        tx_clean_index = wrap_ring(tx_clean_index, self.transmit_ring.len());
                    }

                    count += 1;

                    if tx_clean_index == self.transmit_index || count == batch {
                        break;
                    }
                }
                self.tx_clean_index = wrap_ring(tx_clean_index, self.transmit_ring.len());
            }
        }

        if sent > 0 && tx_index == last_tx_index {
            println!("Queued packets, but failed to update idx");
            println!("last_tx_index {} tx_index {} tx_clean_index {}", last_tx_index, tx_index, tx_clean_index);
        }

        if tx_index != last_tx_index {
            if debug {
                println!("Update tdt from {} to {}", self.read_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0), tx_index);
            }
            //self.bar.write_reg_tdt(0, tx_index as u64);
            self.write_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0, tx_index as u64);
            self.transmit_index = tx_index;
            self.tx_clean_index = tx_clean_index;
        }

        if sent == 0 {
            //println!("Sent {} packets", sent);
        }
        sent
    }

    #[inline(always)]
    fn rx_submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, debug: bool) -> usize {
        let mut rx_index = self.receive_index;
        let mut last_rx_index = self.receive_index;
        let mut received_packets = 0;
        let mut rx_clean_index = self.rx_clean_index;
        let BATCH_SIZE = 32;

        if packets.len() > 0 {
            while let Some(packet) = packets.pop_front() {

                let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_index) as *mut ixgbe_adv_rx_desc) };

                let status = unsafe {
                    core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32) };

                unsafe {
                    //println!("pkt_addr {:08X} status {:x}",
                    //            (*desc).read.pkt_addr as *const u64 as u64, status);
                                //self.receive_buffers[rx_index].physical());
                }

                if debug {
                    println!("rx_index {} clean_index {}", rx_index, rx_clean_index);
                }
                if ((status & IXGBE_RXDADV_STAT_DD) == 0) && self.rx_slot[rx_index] {
                    //println!("no packets to rx");
                    packets.push_front(packet);
                    break;
                }

                if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                    panic!("increase buffer size or decrease MTU")
                }

                // Reset the status DD bit
                /*unsafe {
                    if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                        core::ptr::write_volatile(&mut (*desc).wb.upper.status_error as *mut u32,
                                    status & !IXGBE_RXDADV_STAT_DD);
                    }
                }*/

                //println!("Found packet {}", rx_index);
                let length = unsafe { core::ptr::read_volatile(
                            &(*desc).wb.upper.length as *const u16) as usize
                };

                //if length > 0 {
                   //println!("Got a packet with len {}", length);
                //}

                unsafe {
                    if self.rx_slot[rx_index] {
                        if let Some(pkt) = &mut self.receive_buffers[rx_index] {
                            //let mut buf = pkt.as_mut_ptr();
                            //println!("{:x} len {} cap {}", buf as u64, pkt.len(), pkt.capacity());
                            if length <= pkt.capacity() {
                                let vec = Vec::from_raw_parts(pkt.as_mut_ptr(), length as usize, pkt.capacity());
                                reap_queue.push_back(vec);
                                //received_packets += 1;
                            } else {
                                println!("Not pushed");
                            }
                            self.receive_buffers[rx_index] = None;
                        }
                        self.rx_slot[rx_index] = false;
                        rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());
                    }

                    core::ptr::write_volatile(
                        &(*self.receive_ring.as_ptr().add(rx_index)).read.pkt_addr as *const u64 as *mut u64,
                        packet.as_ptr() as u64);

                    core::ptr::write_volatile(
                        &(*self.receive_ring.as_ptr().add(rx_index)).read.hdr_addr as *const u64 as *mut u64,
                        0 as u64);

                    self.receive_buffers[rx_index] = Some(packet);
                    self.rx_slot[rx_index] = true;
                }

                last_rx_index = rx_index;
                rx_index = wrap_ring(rx_index, self.receive_ring.len());

                received_packets += 1;
            }

            rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());

            if reap_queue.len() < BATCH_SIZE {
                let rx_index = self.receive_index;
                let mut reaped = 0;
                let batch = BATCH_SIZE - reap_queue.len(); 
                let mut count = 0;
                let last_rx_clean = rx_clean_index;
                //print!("reap_queue {} ", reap_queue.len());

                loop {
                    let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_clean_index) as
                                                  *mut ixgbe_adv_rx_desc) };

                    let status = unsafe {
                            core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32)
                    };

                    if debug {
                        println!("checking status[{}] {:x}", rx_clean_index, status);
                    }

                    if ((status & IXGBE_RXDADV_STAT_DD) == 0) {
                        break;
                    }

                    if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                        panic!("increase buffer size or decrease MTU")
                    }

                    if self.rx_slot[rx_clean_index] {
                        if let Some(pkt) = &mut self.receive_buffers[rx_clean_index] {
                            let length = unsafe { core::ptr::read_volatile(
                                    &(*desc).wb.upper.length as *const u16) as usize
                            };

                            let mut buf = pkt.as_mut_ptr();
                            let vec = unsafe { Vec::from_raw_parts(buf, length, pkt.capacity()) };
                            reap_queue.push_back(vec);
                        }
                        self.rx_slot[rx_clean_index] = false;
                        self.receive_buffers[rx_clean_index] = None;
                        reaped += 1;
                        rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());
                    }

                    count += 1;

                    if rx_clean_index == rx_index || count == BATCH_SIZE {
                        break;
                    }
                }

                if debug {
                    println!("clean_index {}", rx_clean_index);
                }

                //print!("reap_queue_after {}\n", reap_queue.len());

                if last_rx_clean != rx_clean_index {
                    rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());
                }
            }
        }

        if rx_index != last_rx_index {
            if debug {
                println!("Update rdt from {} to {}", self.read_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0), last_rx_index);
                println!("rx_index {} clean_index {}", rx_index, self.rx_clean_index);
            }
            self.write_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0, last_rx_index as u64);
            self.receive_index = rx_index;
            self.rx_clean_index = rx_clean_index;
        }

        received_packets
    }

    fn tx_submit_and_poll_rref(&mut self, mut packets: RRefDeque<[u8; 1512], 32>,
                                mut reap_queue: RRefDeque<[u8; 1512], 32>, debug: bool) ->
            (usize, RRefDeque<[u8; 1512], 32>, RRefDeque<[u8; 1512], 32>)
    {
        let mut sent = 0;
        let mut tx_index = self.transmit_index;
        let mut tx_clean_index = self.tx_clean_index;
        let mut last_tx_index = self.transmit_index;
        let num_descriptors = self.transmit_ring.len();
        let BATCH_SIZE = 32;


        if debug {
            //println!("tx index {} packets {}", tx_index, packets.len());
        }

        while let Some(packet) = packets.pop_front() {

            //println!("Found packet!");
            let mut desc = unsafe { &mut*(self.transmit_ring.as_ptr().add(tx_index) as *mut ixgbe_adv_tx_desc) };

            let status = unsafe {
                core::ptr::read_volatile(&mut (*desc).wb.status as *mut u32) };

            unsafe {
                //println!("pkt_addr {:08X} tx_Buffer {:08X}",
                //            (*desc).read.pkt_addr as *const u64 as u64,
                //            self.transmit_buffer[tx_index].physical());
            }

            // DD == 0 on a TX desc leaves us with 2 possibilities
            // 1) The desc is populated (tx_slot[i] = true), the device did not sent it out yet
            // 2) The desc is not populated. In that case, tx_slot[i] = false
            if ((status & IXGBE_RXDADV_STAT_DD) == 0) && self.tx_slot[tx_index] {
                if debug {
                    //println!("No free slot. Fucked");
                    if !self.dump {
                        self.dump_tx_desc();
                    }
                }
                packets.push_back(packet);
                break;
            }

            let pkt_len = 64;
            if debug {
                //println!("packet len {}", pkt_len);
            }
            unsafe {
                if self.tx_slot[tx_index] {
                    if let Some(mut buf) = self.transmit_rrefs[tx_index].take() {
                        if debug {
                            //println!("buf {:x}", buf as u64);
                        }

                        //if reap_queue.push_back(RRef::new(buf.take().unwrap())).is_some() {
                        if reap_queue.push_back(buf).is_some() {
                            //println!("tx_sub_and_poll1: Pushing to a full reap queue");
                        }

                        tx_clean_index = wrap_ring(tx_clean_index, self.transmit_ring.len());
                    }
                }


                let pkt_addr = &*packet as *const [u8; 1512] as *const u64 as u64;
                if debug {
                    //println!("programming new buffer! {:x} packet[0] {:x}", packet.as_ptr() as u64, packet[0]);
                }
                // switch to a new buffer
                core::ptr::write_volatile(
                    &(*self.transmit_ring.as_ptr().add(tx_index)).read.buffer_addr as *const u64 as *mut u64,
                        pkt_addr);

                self.transmit_rrefs[tx_index] = Some(packet);
                self.tx_slot[tx_index] = true;

                core::ptr::write_volatile(
                        &(*self.transmit_ring.as_ptr().add(tx_index)).read.cmd_type_len as *const u32 as *mut u32,
                        IXGBE_ADVTXD_DCMD_EOP
                                | IXGBE_ADVTXD_DCMD_RS
                                | IXGBE_ADVTXD_DCMD_IFCS
                                | IXGBE_ADVTXD_DCMD_DEXT
                                | IXGBE_ADVTXD_DTYP_DATA
                                | pkt_len as u32,
                );

                core::ptr::write_volatile(
                        &(*self.transmit_ring.as_ptr().add(tx_index)).read.olinfo_status as *const u32 as *mut u32,
                        (pkt_len as u32) << IXGBE_ADVTXD_PAYLEN_SHIFT,
                );
            }

            last_tx_index = tx_index;
            tx_index = wrap_ring(tx_index, self.transmit_ring.len());
            sent += 1;
        }
        if reap_queue.len() < BATCH_SIZE {
            let mut reaped = 0;
            let mut count = 0;
            let batch = BATCH_SIZE - reap_queue.len();

            loop {
                let status = unsafe {
                    core::ptr::read_volatile(&(*self.transmit_ring.as_ptr().add(tx_clean_index)).wb.status
                       as *const u32)
                };

                if (status & IXGBE_ADVTXD_STAT_DD) != 0 {
                    if self.tx_slot[tx_clean_index] {
                        if let Some(mut buf) = self.transmit_rrefs[tx_clean_index].take() {
                            if reap_queue.push_back(buf).is_some() {
                                //println!("tx_sub_and_poll2: Pushing to a full reap queue");
                            }
                        }
 
                        self.tx_slot[tx_clean_index] = false;
                        self.transmit_buffers[tx_clean_index] = None;
                        reaped += 1;
                    }
                    tx_clean_index = wrap_ring(tx_clean_index, self.transmit_ring.len());
                }

                count += 1;

                if tx_clean_index == self.transmit_index || count == batch {
                    break;
                }
            }
            self.tx_clean_index = wrap_ring(tx_clean_index, self.transmit_ring.len());
        }

        if sent > 0 && tx_index == last_tx_index {
            //println!("Queued packets, but failed to update idx");
            //println!("last_tx_index {} tx_index {} tx_clean_index {}", last_tx_index, tx_index, tx_clean_index);
        }

        if tx_index != last_tx_index {
            if debug {
               // println!("Update tdt from {} to {}", self.read_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0), tx_index);
            }
            //self.bar.write_reg_tdt(0, tx_index as u64);
            self.write_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0, tx_index as u64);
            self.transmit_index = tx_index;
            self.tx_clean_index = tx_clean_index;
        }

        if sent == 0 {
            //println!("Sent {} packets", sent);
        }
        (sent, packets, reap_queue)
    }

    #[inline(always)]
    fn rx_submit_and_poll_rref(&mut self, mut packets: RRefDeque<[u8; 1512], 32>,
                                mut reap_queue: RRefDeque<[u8; 1512], 32>, debug: bool) ->
            (usize, RRefDeque<[u8; 1512], 32>, RRefDeque<[u8; 1512], 32>)
    {
        let mut rx_index = self.receive_index;
        let mut last_rx_index = self.receive_index;
        let mut received_packets = 0;
        let mut rx_clean_index = self.rx_clean_index;
        let BATCH_SIZE = 32;

        while let Some(packet) = packets.pop_front() {

            let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_index) as *mut ixgbe_adv_rx_desc) };

            let status = unsafe {
                core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32) };

            unsafe {
                //println!("pkt_addr {:08X} status {:x}",
                //            (*desc).read.pkt_addr as *const u64 as u64, status);
                            //self.receive_buffers[rx_index].physical());
            }

            if debug {
                println!("rx_index {} clean_index {}", rx_index, rx_clean_index);
            }
            if ((status & IXGBE_RXDADV_STAT_DD) == 0) && self.rx_slot[rx_index] {
                //println!("no packets to rx");
                packets.push_back(packet);
                break;
            }

            if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                panic!("increase buffer size or decrease MTU")
            }

            // Reset the status DD bit
            /*unsafe {
                if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                    core::ptr::write_volatile(&mut (*desc).wb.upper.status_error as *mut u32,
                                status & !IXGBE_RXDADV_STAT_DD);
                }
            }*/

            //println!("Found packet {}", rx_index);
            let length = unsafe { core::ptr::read_volatile(
                        &(*desc).wb.upper.length as *const u16) as usize
            };

            //if length > 0 {
               //println!("Got a packet with len {}", length);
            //}

            unsafe {
                if self.rx_slot[rx_index] {
                    if let Some(mut buf) = self.receive_rrefs[rx_index].take() {
                        if length <= 1512 {
                            if reap_queue.push_back(buf).is_some() {
                                println!("rx_sub_and_poll1: Pushing to a full reap queue");
                            }
                        } else {
                            println!("Not pushed");
                        }
                    }
                    self.rx_slot[rx_index] = false;
                    rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());
                }

                let pkt_addr = &*packet as *const [u8; 1512] as *const u64 as u64;

                core::ptr::write_volatile(
                    &(*self.receive_ring.as_ptr().add(rx_index)).read.pkt_addr as *const u64 as *mut u64,
                    pkt_addr);

                core::ptr::write_volatile(
                    &(*self.receive_ring.as_ptr().add(rx_index)).read.hdr_addr as *const u64 as *mut u64,
                    0 as u64);

                self.receive_rrefs[rx_index] = Some(packet);
                self.rx_slot[rx_index] = true;
            }

            last_rx_index = rx_index;
            rx_index = wrap_ring(rx_index, self.receive_ring.len());

            received_packets += 1;
        }

        rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());

        if reap_queue.len() < BATCH_SIZE {
            let rx_index = self.receive_index;
            let mut reaped = 0;
            let batch = BATCH_SIZE - reap_queue.len(); 
            let mut count = 0;
            let last_rx_clean = rx_clean_index;
            //print!("reap_queue {} ", reap_queue.len());

            loop {
                let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_clean_index) as
                                              *mut ixgbe_adv_rx_desc) };

                let status = unsafe {
                        core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32)
                };

                if debug {
                    println!("checking status[{}] {:x}", rx_clean_index, status);
                }

                if ((status & IXGBE_RXDADV_STAT_DD) == 0) {
                    break;
                }

                if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                    panic!("increase buffer size or decrease MTU")
                }

                if self.rx_slot[rx_clean_index] {
                    if let Some(mut pkt) = self.receive_rrefs[rx_clean_index].take() {
                        let length = unsafe { core::ptr::read_volatile(
                                &(*desc).wb.upper.length as *const u16) as usize
                        };

                        if reap_queue.push_back(pkt).is_some() {
                            println!("rx_sub_and_poll2: Pushing to a full reap queue");
                        }
                    }
                    self.rx_slot[rx_clean_index] = false;
                    self.receive_rrefs[rx_clean_index] = None;
                    reaped += 1;
                    rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());
                }

                count += 1;

                if rx_clean_index == rx_index || count == batch {
                    break;
                }
            }

            if debug {
                println!("clean_index {}", rx_clean_index);
            }

            //print!("reap_queue_after {}\n", reap_queue.len());

            if last_rx_clean != rx_clean_index {
                rx_clean_index = wrap_ring(rx_clean_index, self.receive_ring.len());
            }
        }

        if rx_index != last_rx_index {
            if debug {
                println!("Update rdt from {} to {}", self.read_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0), last_rx_index);
                println!("rx_index {} clean_index {}", rx_index, self.rx_clean_index);
            }
            self.write_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0, last_rx_index as u64);
            self.receive_index = rx_index;
            self.rx_clean_index = rx_clean_index;
        }

        (received_packets, packets, reap_queue)
    }

    pub fn dump_dma_regs(&self) {

        let mut string = format!("Interrupt regs:\n\tEITR {:08X} IVAR(0) {:08X}\n",
                    self.read_qreg_idx(IxgbeDmaArrayRegs::Eitr, 0) as u32,
                    self.read_qreg_idx(IxgbeDmaArrayRegs::Ivar, 0) as u32);


        string.push_str(&format!("Receive regs:\n\tRXPBSIZE(0): {:08X} SRRCTL(0) {:08X}\n\tRDBAL(0) {:08X} RDBAH(0) {:08X} \
                                 \n\tRDLEN(0) {:08X} RDH(0) {:08X} RDT(0) {:08X}\n",
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Rxpbsize, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Srrctl, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Rdbal, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Rdbah, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Rdlen, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Rdh, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Rdt, 0) as u32));

        string.push_str(&format!("Transmit regs:\n\tTXDCTL(0) {:08X} TXPBSIZE(0): {:08X}\n\t \
                                 TDBAL(0) {:08X} TDBAH(0) {:08X}\n\t \
                                 TDLEN(0) {:08X} TDH(0) {:08X} TDT(0) {:08X}\n",
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Txdctl, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Txpbsize, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Tdbal, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Tdbah, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Tdlen, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Tdh, 0) as u32,
                                 self.read_qreg_idx(IxgbeDmaArrayRegs::Tdt, 0) as u32));

        print!("{}", string);
    }
}
