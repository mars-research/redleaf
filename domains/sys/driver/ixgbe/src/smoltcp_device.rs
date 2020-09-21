// An implementation of smoltcp::phy::Device for ixgbe
//
// its 2am and im trying to cook up some lousy demo to make myself happy
// and get slapped for wasting time for no reason so cut me some slack thank u

extern crate smoltcp;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::rc::Rc;
use core::cell::{RefCell, RefMut};
use core::borrow::BorrowMut;

use crate::device::Intel8259x;
use console::println;

use smoltcp::phy::{Device, DeviceCapabilities, ChecksumCapabilities, Checksum, RxToken, TxToken};
use smoltcp::time::Instant;
use smoltcp::Result as SmolResult;
use smoltcp::Error as SmolError;

const BATCH_SZ: usize = 32;

pub struct SmolIxgbe {
    pool: Rc<RefCell<VecDeque<Vec<u8>>>>,
    rx: Rc<RefCell<VecDeque<Vec<u8>>>>,
    tx: Rc<RefCell<VecDeque<Vec<u8>>>>,
    dev: Rc<RefCell<Intel8259x>>,
}

impl SmolIxgbe {
    pub fn new(dev: Intel8259x) -> Self {
        let mut boi = Self {
            pool: Rc::new(RefCell::new(VecDeque::with_capacity(BATCH_SZ))),
            rx: Rc::new(RefCell::new(VecDeque::with_capacity(BATCH_SZ))),
            tx: Rc::new(RefCell::new(VecDeque::with_capacity(BATCH_SZ))),
            dev: Rc::new(RefCell::new(dev)),
        };

        {
            let mut pool: RefMut<VecDeque<Vec<u8>>> = (*boi.pool).borrow_mut();

            for i in 0..BATCH_SZ {
                pool.push_front(Vec::with_capacity(2048));
            }
        }

        boi
    }

    // do_rx() -> execute smoltcp functions -> do_tx()

    pub fn do_rx(&mut self) {
        let mut pool = (*self.pool).borrow_mut();
        let mut rx = (*self.rx).borrow_mut();
        let mut dev = (*self.dev).borrow_mut();

        dev.device.submit_and_poll(&mut pool, &mut rx, false, false);
    }
    
    pub fn do_tx(&mut self) {
        let mut pool = (*self.pool).borrow_mut();
        let mut tx = (*self.tx).borrow_mut();
        let mut dev = (*self.dev).borrow_mut();

        /*
        if tx.len() != 0 {
            for (i, f) in tx.iter().enumerate() {
                println!("txq {}: {:x?}", i, f);
            }
        }
        */

        dev.device.submit_and_poll(&mut tx, &mut pool, true, false);

        if pool.len() == 0 && tx.len() < BATCH_SZ * 4 {
            for i in 0..BATCH_SZ {
                pool.push_front(Vec::with_capacity(2048));
            }
        }
    }

    fn get_tx_frame(&self) -> IxgbeTxToken {
        let tx_frame = {
            let mut pool: RefMut<VecDeque<Vec<u8>>> = (*self.pool).borrow_mut();

            match pool.pop_front() {
                Some(frame) => frame,
                None => Vec::with_capacity(2048),
            }
        };
        IxgbeTxToken {
            frame: Some(tx_frame),
            tx: Rc::clone(&self.tx),
            pool: Rc::clone(&self.pool),
        }
    }
}

impl<'a> Device<'a> for SmolIxgbe {
    type RxToken = IxgbeRxToken;
    type TxToken = IxgbeTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        // we are taking two buffers for each rx right now lol
        let mut rx = (*self.rx).borrow_mut();

        match rx.pop_front() {
            Some(frame) => {
                // we have some packet!
                let rx_token = IxgbeRxToken {
                    frame: frame,
                    pool: Rc::clone(&self.pool),
                };
                let tx_token = self.get_tx_frame();

                Some((rx_token, tx_token))
            },
            None => None,
        }
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

pub struct IxgbeTxToken {
    frame: Option<Vec<u8>>,
    tx: Rc<RefCell<VecDeque<Vec<u8>>>>,
    pool: Rc<RefCell<VecDeque<Vec<u8>>>>,
}

impl TxToken for IxgbeTxToken {
    // consume the cum chalice
    fn consume<R, F>(mut self, _timestamp: Instant, len: usize, f: F) -> SmolResult<R>
        where F: FnOnce(&mut [u8]) -> SmolResult<R>
    {
        match self.frame.take() {
            Some(mut frame) => {
                unsafe {
                    frame.set_len(len);
                }
                let result = f(&mut frame);
                
                let mut tx = (*self.tx).borrow_mut();
                tx.push_back(frame);

                result
            },
            None => Err(SmolError::Illegal),
        }
    }
}

impl Drop for IxgbeTxToken {
    fn drop(&mut self) {
        if let Some(frame) = self.frame.take() {
            let mut pool = (*self.pool).borrow_mut();
            pool.push_back(frame);
        }
    }
}

pub struct IxgbeRxToken {
    frame: Vec<u8>,
    pool: Rc<RefCell<VecDeque<Vec<u8>>>>,
}

impl RxToken for IxgbeRxToken {
    // consume the cum chalice
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> SmolResult<R>
        where F: FnOnce(&mut [u8]) -> SmolResult<R>
    {
        let result = f(&mut self.frame);
        
        let mut pool = (*self.pool).borrow_mut();
        pool.push_back(self.frame);

        result
    }
}
