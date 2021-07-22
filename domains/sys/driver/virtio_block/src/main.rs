#![no_std]
#![no_main]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    const_in_array_repeat_expressions,
    untagged_unions,
    maybe_uninit_extra,
    assoc_char_funcs
)]

extern crate alloc;
extern crate malloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{boxed::Box, collections::BTreeMap};
use core::{borrow::BorrowMut, panic::PanicInfo, pin::Pin, usize};
use nullblk::NullBlk;
use syscalls::{Heap, Syscall};

use console::{print, println};
use interface::bdev::BlkReq;
use interface::bdev::NvmeBDev;
use interface::bdev::BSIZE;
pub use interface::error::{ErrorKind, Result};
use interface::rref::{RRef, RRefDeque};
use interface::{net::Net, rpc::RpcResult};
use libsyscalls::syscalls::sys_backtrace;
pub use platform::PciBarAddr;
use spin::Mutex;
use virtio_block_device::pci::PciFactory;
use virtio_block_device::VirtioBlockInner;

mod nullblk;
pub struct VirtioBlock(Arc<Mutex<VirtioBlockInner>>);

impl interface::bdev::NvmeBDev for VirtioBlock {
    fn submit_and_poll_rref(
        &self,
        mut submit: RRefDeque<BlkReq, 128>,
        mut collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)>> {
        println!("VIRTIO BLOCK: submit_and_poll_rref");
        let mut device = self.0.lock();

        while let Some(buffer) = submit.pop_front() {
            device.submit_request(buffer, write);
        }

        let count = device.free_request_buffers(&mut collect);

        assert_eq!(submit.len(), 0);
        println!("FJFJFJ: {}", submit.len());

        Ok(Ok((count, submit, collect)))
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<BlkReq, 1024>,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>> {
        unimplemented!();
    }

    fn get_stats(&self) -> RpcResult<Result<(u64, u64)>> {
        println!("Virtio Block: get_stats()");
        unimplemented!();
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    pci: Box<dyn interface::pci::PCI>,
) -> Box<dyn interface::bdev::NvmeBDev> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    // #[cfg(feature = "virtio_block")]
    // println!("Virtio Block starting");

    #[cfg(feature = "virtio_block")]
    let blk = {
        let blk = {
            let mut pci_factory = PciFactory::new();
            if pci.pci_register_driver(&mut pci_factory, 4, None).is_err() {
                panic!("Failed to probe VirtioBlock PCI");
            }
            let dev = pci_factory.to_device().unwrap();
            VirtioBlock(Arc::new(Mutex::new(dev)))
        };
        blk.0.lock().init();
        blk
    };

    #[cfg(not(feature = "virtio_block"))]
    let blk = NullBlk::new();

    // Testing Code

    // let mut submit = RRefDeque::new([None; 128]);
    // let mut collect = RRefDeque::new([None; 128]);

    // // Write requests to `submit`
    // for i in 0..(300 as u16) {
    //     let req = BlkReq {
    //         data: [(i % 20 + 33) as u8; 4096],
    //         data_len: 4096,
    //         block: (i % 20) as u64,
    //     };

    //     println!(
    //         "Writing {:} to sector {:}",
    //         char::from_u32((i % 20 + 33) as u32).unwrap(),
    //         i % 20
    //     );

    //     libtime::sys_ns_sleep(9999999);

    //     submit.push_back(RRef::new(req));
    //     let res = blk
    //         .submit_and_poll_rref(submit, collect, true)
    //         .unwrap()
    //         .unwrap();
    //     submit = res.1;
    //     collect = res.2;

    //     // Clear out collect
    //     while let Some(_) = collect.pop_front() {}
    // }

    // // Read back and check
    // for i in 0..(20 as u16) {
    //     let req = BlkReq {
    //         data: [0xFF; 4096],
    //         data_len: 4096,
    //         block: (i % 20) as u64,
    //     };

    //     libtime::sys_ns_sleep(9999999);

    //     submit.push_back(RRef::new(req));
    //     let res = blk
    //         .submit_and_poll_rref(submit, collect, false)
    //         .unwrap()
    //         .unwrap();
    //     submit = res.1;
    //     collect = res.2;

    //     while let Some(block) = collect.pop_front() {
    //         println!("{:} == {:}?", block.data[0], block.block % 20 + 33);
    //         assert_eq!(
    //             block.data[0],
    //             (block.block % 20 + 33) as u8,
    //             "block.data: {:} != block.block % 20 + 33 {:}",
    //             block.data[0],
    //             block.block % 20 + 33
    //         );
    //     }

    //     // Clear out collect
    //     while let Some(_) = collect.pop_front() {}
    // }

    // println!("Block Test Complete!");
    // loop {}

    println!("Virtio Block: trusted_entry()");

    Box::new(blk)
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    sys_backtrace();
    loop {}
}
