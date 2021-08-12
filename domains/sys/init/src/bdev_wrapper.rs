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
        let wrapper = BDevWrapperInner {
            nvme,
            submit: Some(RRefDeque::new([None; 128])),
            collect: Some(RRefDeque::new([None; 128])),
            request: Some(RRef::new(BlkReq {
                block: 0,
                data: [0; 4096],
                data_len: 4096,
            })),
        };

        Self {
            inner: Arc::new(Mutex::new(wrapper)),
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
        let mut req = self.request.take().unwrap();
        req.block = block as u64;
        req.data = *data;

        self.submit.as_mut().unwrap().push_back(req);

        println!("Block {}: Started", &block);
        println!("submit.len(): {}", self.submit.as_ref().unwrap().len());

        match self.nvme.submit_and_poll_rref(
            self.submit.take().unwrap(),
            self.collect.take().unwrap(),
            false,
        ) {
            Ok(Ok((_, submit, collect))) => {
                self.submit.replace(submit);
                self.collect.replace(collect);
                println!("Block {}: Request Submitted", &block);
            }
            Err(e) => {
                panic!("BDevWrapper RpcError {:?}", e);
            }
            Ok(Err(e)) => {
                panic!("BDevWrapper Other Error {:?}", e);
            }
        }

        println!(
            "submit.len(): {}, collect.len(): {}",
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
                Ok(Ok((_, submit, mut collect))) => {
                    while let Some(req) = collect.pop_front() {
                        println!("Read Block {}: Done", &req.block);

                        let data = req.data;

                        self.submit.replace(submit);
                        self.collect.replace(collect);
                        self.request.replace(req);

                        return Ok(RRef::new(data));
                    }

                    println!("Read Block {}: Not Done", &block);
                    println!("Read Collect Len: {}", collect.len());
                    self.submit.replace(submit);
                    self.collect.replace(collect);
                }
                Err(e) => {
                    panic!("BDevWrapper RpcError {:?}", e);
                }
                Ok(Err(e)) => {
                    panic!("BDevWrapper Other Error {:?}", e);
                }
            }
        }
    }

    fn write(&mut self, block: u32, data: &RRef<[u8; 4096]>) -> RpcResult<()> {
        // Modify the request
        let mut req = self.request.take().unwrap();
        req.block = block as u64;
        // req.data = **data;
        req.data = [0x69; 4096];

        // println!("block: {}, req.block: {}", block, req.block);
        // println!("data: {:?}, req.data: {:?}", **data, req.data);

        self.submit.as_mut().unwrap().push_back(req);

        println!("Write Block {}: Started", &block);
        println!("Write submit.len(): {}", self.submit.as_ref().unwrap().len());

        // Wait for it to finish and return
        loop {
            match self.nvme.submit_and_poll_rref(
                self.submit.take().unwrap(),
                self.collect.take().unwrap(),
                true,
            ) {
                Ok(Ok((_, submit, mut collect))) => {
                    while let Some(req) = collect.pop_front() {
                        println!("Write Block {}: Done", &req.block);

                        self.submit.replace(submit);
                        self.collect.replace(collect);
                        self.request.replace(req);

                        let data = RRef::new([0x42; 4096]);
                        if let Ok(res) = self.read(block, data) {
                            println!("Write data: {:?}", *res);
                        }

                        return Ok(());
                    }

                    println!("Write Block {}: Not Done", &block);
                    println!("Write Collect Len: {}", collect.len());
                    self.submit.replace(submit);
                    self.collect.replace(collect);
                }
                Err(e) => {
                    panic!("BDevWrapper RpcError {:?}", e);
                }
                Ok(Err(e)) => {
                    panic!("BDevWrapper Other Error {:?}", e);
                }
            }
        }
    }
}
