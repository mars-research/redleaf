// An implementation of smoltcp::phy::Device for RedLeaf Net

extern crate smoltcp;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::boxed::Box;
// use core::cell::{Mutex, RefMut};
use core::borrow::BorrowMut;

use alloc::sync::Arc;
use spin::Mutex;

use console::println;

use smoltcp::phy::{Device, DeviceCapabilities, ChecksumCapabilities, Checksum, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::Result as SmolResult;
use smoltcp::Error as SmolError;

use usr_interface::net::Net;
use rref::{RRef, RRefDeque};
use core::default::Default;
use core::ops::Deref;

const BATCH_SZ: usize = 32;

type PhyFrame = RRef<[u8; 1514]>;
type PhyQueue = RRefDeque<[u8; 1514], 32>;

pub struct SmolPhy {
    pool: Arc<Mutex<Option<PhyQueue>>>,
    rx: Arc<Mutex<Option<PhyQueue>>>,
    tx: Arc<Mutex<Option<PhyQueue>>>,
    phy: Box<dyn Net>,
}

impl SmolPhy {
    pub fn new(phy: Box<dyn Net>) -> Self {
        let mut pool: PhyQueue = Default::default();
        let rx: PhyQueue = Default::default();
        let tx: PhyQueue = Default::default();

        for i in 0..BATCH_SZ {
            pool.push_back(PhyFrame::new([0; 1514]));
        }

        Self {
            pool: Arc::new(Mutex::new(Some(pool))),
            rx: Arc::new(Mutex::new(Some(rx))),
            tx: Arc::new(Mutex::new(Some(tx))),
            phy,
        }
    }


    // do_rx() -> execute smoltcp functions -> do_tx()

    pub fn do_rx(&mut self) {
        // FIXME: Move this monstrosity into a callback-based method
        // Maybe self.take_all(Fn<(rx, tx, pool)> -> (rx, tx, pool))?
        let mut pool_guard = self.pool.lock();
        let mut rx_guard = self.rx.lock();

        let mut pool = pool_guard.take().unwrap();
        let mut rx = rx_guard.take().unwrap();

        // FIXME: Fixed packet length???????? Why?
        let (_, pool, rx) = self.phy.submit_and_poll_rref(pool, rx, false, 1514).unwrap().unwrap(); // RpcResult<Result<T>>

        pool_guard.replace(pool);
        rx_guard.replace(rx);
    }
    
    pub fn do_tx(&mut self) {
        let mut pool_guard = self.pool.lock();
        let mut tx_guard = self.tx.lock();

        let mut pool = pool_guard.take().unwrap();
        let mut tx = tx_guard.take().unwrap();

        /*
        if tx.len() != 0 {
            for (i, f) in tx.iter().enumerate() {
                println!("txq {}: {:x?}", i, f);
            }
        }
        */

        let (_, tx, mut pool) = self.phy.submit_and_poll_rref(tx, pool, true, 1514).unwrap().unwrap(); // RpcResult<Result<T>>

        if pool.len() == 0 && tx.len() < BATCH_SZ * 4 {
            for i in 0..BATCH_SZ {
                pool.push_back(PhyFrame::new([0; 1514]));
            }
        }

        pool_guard.replace(pool);
        tx_guard.replace(tx);
    }

    fn get_tx_frame(&self) -> SmolPhyTxToken {
        let tx_frame = {
            let mut pool_guard = self.pool.lock();
            let mut pool = pool_guard.take().unwrap();

            let frame = match pool.pop_front() {
                Some(frame) => frame,
                None => PhyFrame::new([0; 1514]),
            };

            pool_guard.replace(pool);

            frame
        };
        SmolPhyTxToken {
            frame: Some(tx_frame),
            tx: Arc::clone(&self.tx),
            pool: Arc::clone(&self.pool),
        }
    }
}

impl<'a> Device<'a> for SmolPhy {
    type RxToken = SmolPhyRxToken;
    type TxToken = SmolPhyTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        // we are taking two buffers for each rx right now lol
        let mut rx_guard = self.rx.lock();
        let mut rx = rx_guard.take().unwrap();

        let r = match rx.pop_front() {
            Some(frame) => {
                // we have some packet!
                let rx_token = SmolPhyRxToken {
                    frame,
                    pool: Arc::clone(&self.pool),
                };
                let tx_token = self.get_tx_frame();

                Some((rx_token, tx_token))
            },
            None => None,
        };

        rx_guard.replace(rx);
        r
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(self.get_tx_frame())
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut cap = DeviceCapabilities::default();

        cap.max_transmission_unit = 2048;
        cap.max_burst_size = None;
        cap.checksum = ChecksumCapabilities::ignored();
        cap.checksum.ipv4 = Checksum::Tx;
        cap.checksum.tcp = Checksum::Tx;

        cap
    }
}

pub struct SmolPhyTxToken {
    frame: Option<PhyFrame>,
    tx: Arc<Mutex<Option<PhyQueue>>>,
    pool: Arc<Mutex<Option<PhyQueue>>>,
}

impl TxToken for SmolPhyTxToken {
    // consume the cum chalice
    fn consume<R, F>(mut self, _timestamp: Instant, len: usize, f: F) -> SmolResult<R>
        where F: FnOnce(&mut [u8]) -> SmolResult<R>
    {
        match self.frame.take() {
            Some(mut frame) => {
                // FIXME: Why can't I set lengths for individual packets? How
                // is this usable?

                // unsafe {
                //     frame.set_len(len);
                // }
                let result = f(&mut *frame);
                
                {
                    let mut tx_guard = self.tx.lock();
                    let mut tx = tx_guard.take().unwrap();
                    tx.push_back(frame);
                    tx_guard.replace(tx);
                }

                result
            },
            None => Err(SmolError::Illegal),
        }
    }
}

impl Drop for SmolPhyTxToken {
    fn drop(&mut self) {
        if let Some(frame) = self.frame.take() {
            let mut pool_guard = self.pool.lock();
            let mut pool = pool_guard.take().unwrap();

            pool.push_back(frame);

            pool_guard.replace(pool);
        }
    }
}

pub struct SmolPhyRxToken {
    frame: PhyFrame,
    pool: Arc<Mutex<Option<PhyQueue>>>,
}

impl RxToken for SmolPhyRxToken {
    // consume the cum chalice
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> SmolResult<R>
        where F: FnOnce(&mut [u8]) -> SmolResult<R>
    {
        let result = f(&mut *self.frame);

        let mut pool_guard = self.pool.lock();
        let mut pool = pool_guard.take().unwrap();
        
        pool.push_back(self.frame);
        pool_guard.replace(pool);

        result
    }
}
