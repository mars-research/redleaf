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
use alloc::boxed::Box;
use alloc::sync::Arc;
use console::print;
use console::println;
use interface::bdev::BDev;
use interface::bdev::BlkReq;
use interface::bdev::NvmeBDev;
use interface::rpc::RpcResult;
use interface::rref::RRef;
use interface::rref::RRefDeque;
use spin::Mutex;

struct BDevWrapperInner {
    nvme: Box<dyn NvmeBDev>,
    submit: Option<RRefDeque<BlkReq, 128>>,
    collect: Option<RRefDeque<BlkReq, 128>>,
    request: Option<RRef<BlkReq>>,
}

pub struct BDevWrapper {
    inner: Arc<Mutex<BDevWrapperInner>>,
}

impl BDevWrapper {
    pub fn new(nvme: Box<dyn NvmeBDev>) -> Self {
        println!("{}", nvme.get_stats().is_ok());
        Self {
            inner: Arc::new(Mutex::new(BDevWrapperInner {
                nvme,
                submit: Some(RRefDeque::new([None; 128])),
                collect: Some(RRefDeque::new([None; 128])),
                request: Some(RRef::new(BlkReq {
                    block: 0,
                    data: [0; 4096],
                    data_len: 4096,
                })),
            })),
        }
    }
}

impl BDev for BDevWrapper {
    fn read(&self, block: u32, data: RRef<[u8; 4096]>) -> RpcResult<RRef<[u8; 4096]>> {
        self.inner.lock().read(block, data)
    }

    fn write(&self, block: u32, data: &RRef<[u8; 4096]>) -> RpcResult<()> {
        self.inner.lock().write(block, data)
    }
}

impl BDevWrapperInner {
    fn read(&mut self, block: u32, data: RRef<[u8; 4096]>) -> RpcResult<RRef<[u8; 4096]>> {
        // Modify the request
        let mut req = self.request.as_mut().unwrap();
        req.block = block as u64;
        req.data = *data;

        self.submit
            .as_mut()
            .unwrap()
            .push_back(self.request.take().unwrap());

        println!("Block: {}", &block);

        match self.nvme.submit_and_poll_rref(
            self.submit.take().unwrap(),
            self.collect.take().unwrap(),
            false,
        ) {
            Ok(Ok((_, submit, collect))) => {
                println!("ASFSDF: {}, {}", submit.len(), collect.len());

                self.submit.replace(submit);
                self.collect.replace(collect);
                println!("{} request submitted", &block);
            }
            Err(e) => {
                panic!("BDevWrapper Failed {:?}", e);
            }
            Ok(Err(e)) => {
                panic!("BDevWrapper Failed 2 {:?}", e);
            }
        }

        println!(
            "submit: {}, collect: {}",
            self.submit.as_ref().unwrap().len(),
            self.collect.as_ref().unwrap().len()
        );

        // Wait for it to finish and return
        loop {
            match self.nvme.submit_and_poll_rref(
                self.submit.take().unwrap(),
                self.collect.take().unwrap(),
                false,
            ) {
                Ok(Ok((count, submit, mut collect))) => {
                    println!("count: {}", &count);

                    while collect.len() != 0 {
                        println!("count 2: {}", collect.len());
                        let req = collect.pop_front();

                        if req.is_some() {
                            let req = req.unwrap();

                            println!("{} done", &block);

                            self.submit.replace(submit);
                            self.collect.replace(collect);
                            return Ok(RRef::new(req.data));
                        }
                    }

                    println!("{} not done", &block);
                    self.submit.replace(submit);
                    self.collect.replace(collect);
                }
                Err(e) => {
                    panic!("BDevWrapper Failed {:?}", e);
                }
                Ok(Err(e)) => {
                    panic!("BDevWrapper Failed 2 {:?}", e);
                }
            }
        }
    }

    fn write(&self, block: u32, data: &RRef<[u8; 4096]>) -> RpcResult<()> {
        unimplemented!();
    }
}
