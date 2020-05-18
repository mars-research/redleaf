#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::boxed::Box;
use array_init::array_init;
use console::{println, print};
use core::mem;
use ixgbe::{IxgbeBarRegion, IxgbeRegs, IxgbeArrayRegs};
use libdma::Dma;
use libdma::ixgbe::{allocate_dma, ixgbe_adv_rx_desc, ixgbe_adv_tx_desc};

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

pub struct IxgbeDevice {
    pub bar: Box<dyn IxgbeBarRegion>,
    transmit_buffers: [Option<Vec<u8>>; 512],
    transmit_ring: Dma<[ixgbe_adv_tx_desc; 512]>,
    receive_buffers: [Option<Vec<u8>>; 512],
    receive_ring: Dma<[ixgbe_adv_rx_desc; 512]>,
    tx_slot: [bool; 512],
    rx_slot: [bool; 512],
    transmit_index: usize,
    transmit_clean_index: usize,
    receive_index: usize,
}

fn wrap_ring(index: usize, ring_size: usize) -> usize {
    (index + 1) & (ring_size - 1)
}

impl IxgbeDevice {

    pub fn new(bar: Box<dyn IxgbeBarRegion>) -> IxgbeDevice {
        IxgbeDevice {
            bar,
            transmit_buffers: array_init::array_init(|_| None),
            receive_buffers: array_init::array_init(|_| None),
            transmit_index: 0,
            transmit_clean_index: 0,
            receive_index: 0,
            tx_slot: [false; 512],
            rx_slot: [false; 512],
            receive_ring: allocate_dma().unwrap(),
            transmit_ring: allocate_dma().unwrap(),

        }
    }

    pub fn start_rx_queue(&self, queue_id: u16) {
        self.bar.write_reg_idx(IxgbeArrayRegs::Rdh, u64::from(queue_id), 0);
        self.bar.write_reg_idx(IxgbeArrayRegs::Rdt, u64::from(queue_id), 0);
    }

    pub fn start_tx_queue(&self, queue_id: u16) {
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdh, u64::from(queue_id), 0);
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdt, u64::from(queue_id), 0);
    }

    pub fn init_rx(&self) {
        let i: u64 = 0;
        self.bar.write_reg_idx(
            IxgbeArrayRegs::Srrctl, i,
            (self.bar.read_reg_idx(IxgbeArrayRegs::Srrctl, i) & !IXGBE_SRRCTL_DESCTYPE_MASK)
                | IXGBE_SRRCTL_DESCTYPE_ADV_ONEBUF,
        );

        // let nic drop packets if no rx descriptor is available instead of buffering them
        self.bar.write_reg_idx(
            IxgbeArrayRegs::Srrctl, i,
            self.bar.read_reg_idx(IxgbeArrayRegs::Srrctl, i) | IXGBE_SRRCTL_DROP_EN,
        );

        self.bar.write_reg_idx(IxgbeArrayRegs::Rdbal, i, (self.receive_ring.physical() & 0xffff_ffff) as u64);

        self.bar.write_reg_idx(IxgbeArrayRegs::Rdbah, i, (self.receive_ring.physical() >> 32) as u64);

        println!("rx ring {} phys addr: {:#x}", i, self.receive_ring.physical());

        self.bar.write_reg_idx(
            IxgbeArrayRegs::Rdlen, i,
            (self.receive_ring.len() * mem::size_of::<ixgbe_adv_rx_desc>()) as u64,
        );
    }

    pub fn init_tx(&self) {
        let i: u64 = 0;
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdbal, i,
                                self.transmit_ring.physical() as u64);
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdbah, i,
                               (self.transmit_ring.physical() >> 32) as u64);

        println!("tx ring {} phys addr: {:#x}", i, self.transmit_ring.physical());
        self.bar.write_reg_idx(IxgbeArrayRegs::Tdlen, i,
            (self.transmit_ring.len() * mem::size_of::<ixgbe_adv_tx_desc>()) as u64
        );
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
            self.bar.write_reg_tdt(0, self.transmit_index as u64);
        }

        sent
    }

    pub fn submit_and_poll(&self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>, tx: bool) -> usize {
        if tx {
            self.tx_submit_and_poll(packets, reap_queue)
        } else {
            self.rx_submit_and_poll(packets, reap_queue)
        }
    }

    pub fn poll(&self,  reap_queue: &mut VecDeque<Vec<u8>>) -> usize {
        let num_descriptors = self.transmit_ring.len();
        let mut reaped: usize = 0;
        let mut count: usize = 0;

        for tx_index in 0..num_descriptors {
            let status = unsafe {
                core::ptr::read_volatile(&(*self.transmit_ring.as_ptr().add(tx_index)).wb.status
                   as *const u32)
            };

            if (status & IXGBE_ADVTXD_STAT_DD) != 0 {
                count += 1;
                if self.tx_slot[tx_index] {
                    if let Some(pkt) = &mut self.transmit_buffers[tx_index] {
                        let mut buf = pkt.as_mut_ptr();
                        let vec = unsafe { Vec::from_raw_parts(buf, pkt.len(), pkt.capacity()) };
                        reap_queue.push_front(vec);
                    }
                    self.tx_slot[tx_index] = false;
                    reaped += 1;
                }
            }
        }
        println!("Found {} sent DDs", count);
        reaped
    }

    #[inline(always)]
    fn tx_submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>) -> usize {
        let mut sent = 0;
        let mut tx_index = self.transmit_index;
        let mut last_tx_index = self.transmit_index;
        let num_descriptors = self.transmit_ring.len();

        //println!("tx index {} packets {}", tx_index, packets.len());
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
                packets.push_front(packet);
                break;
            }

            let pkt_len = packet.len();
            //println!("packet len {}", pkt_len);

            unsafe {
                if self.tx_slot[tx_index] {
                    if let Some(pkt) = &mut self.transmit_buffers[tx_index] {
                        let mut buf = pkt.as_mut_ptr();
                        let vec = Vec::from_raw_parts(buf, pkt.len(), pkt.capacity());
                        reap_queue.push_front(vec);
                    }
                }

                //println!("programming new buffer! {:x}", packet.data.physical());
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

        if tx_index != last_tx_index {
            //println!("Update tdt from {} to {}", self.bar.read_reg_idx(IxgbeArrayRegs::Tdt, 0), tx_index);
            self.bar.write_reg_tdt(0, tx_index as u64);
            self.transmit_index = tx_index;
        }

        if sent > 0 {
            //println!("Sent {} packets", sent);
        }
        sent
    }

    #[inline(always)]
    fn rx_submit_and_poll(&mut self, packets: &mut VecDeque<Vec<u8>>, reap_queue: &mut VecDeque<Vec<u8>>) -> usize {
        let mut rx_index;
        let mut last_rx_index;
        let mut received_packets = 0;

        {
            rx_index = self.receive_index;
            last_rx_index = self.receive_index;

            while let Some(packet) = packets.pop_front() {

                let mut desc = unsafe { &mut*(self.receive_ring.as_ptr().add(rx_index) as *mut ixgbe_adv_rx_desc) };

                let status = unsafe {
                    core::ptr::read_volatile(&mut (*desc).wb.upper.status_error as *mut u32) };

                unsafe {
                    //println!("pkt_addr {:08X} status {:x}",
                    //            (*desc).read.pkt_addr as *const u64 as u64, status);
                                //self.receive_buffers[rx_index].physical());
                }

                if ((status & IXGBE_RXDADV_STAT_DD) == 0) && self.rx_slot[rx_index] {
                    packets.push_front(packet);
                    break;
                }

                if ((status & IXGBE_RXDADV_STAT_DD) != 0) && ((status & IXGBE_RXDADV_STAT_EOP) == 0) {
                    panic!("increase buffer size or decrease MTU")
                }

                // Reset the status DD bit
                unsafe {
                    if (status & IXGBE_RXDADV_STAT_DD) != 0 {
                        core::ptr::write_volatile(&mut (*desc).wb.upper.status_error as *mut u32,
                                    status & !IXGBE_RXDADV_STAT_DD);
                    }
                }

                //println!("Found packet {}", rx_index);
                let length = unsafe { core::ptr::read_volatile(
                            &(*desc).wb.upper.length as *const u16) as isize
                };

                if length > 0 {
                   //println!("Got a packet with len {}", length);
                }

                unsafe {
                    if self.rx_slot[rx_index] {
                        if let Some(pkt) = &mut self.receive_buffers[rx_index] {
                            let mut buf = pkt.as_mut_ptr();
                            if length <= pkt.capacity() as isize {
                                let vec = Vec::from_raw_parts(buf, pkt.len(), pkt.capacity());
                                reap_queue.push_front(vec);
                            }
                        }
                    }

                    core::ptr::write_volatile(
                        &(*self.receive_ring.as_ptr().add(rx_index)).read.pkt_addr as *const u64 as *mut u64,
                        packet.as_ptr() as u64);

                    self.receive_buffers[rx_index] = Some(packet);
                    self.rx_slot[rx_index] = true;
                }

                last_rx_index = rx_index;
                rx_index = wrap_ring(rx_index, self.receive_ring.len());
                received_packets += 1;
            }
        }

        if rx_index != last_rx_index {
            //println!("Update rdt from {} to {}", self.bar.read_reg_idx(IxgbeArrayRegs::Rdt, 0), last_rx_index);
            self.bar.write_reg_rdt(0, last_rx_index as u64);
            self.receive_index = rx_index;
        }

        if received_packets > 0 {
            //println!("Received {} packets", received_packets);
        }
        received_packets
    }

}


