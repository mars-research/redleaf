use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use console::println;
use create::CreateXv6Usr;
use rref::RRefDeque;
use usr_interface::bdev::{BlkReq, NvmeBDev};
use usr_interface::net::{Net, NetworkStats};
use usr_interface::rpc::RpcResult;
use usr_interface::vfs::{FileMode, FileStat, Result, UsrVFS, VFSPtr, NFILE, VFS};
use usr_interface::xv6::{Thread, Xv6};

pub struct Rv6Syscalls {
    create_xv6usr: Arc<dyn CreateXv6Usr + Send + Sync>,
    fs: VFSPtr,
    net: Arc<Mutex<Box<dyn Net + Send>>>,
    nvme: Arc<Mutex<Box<dyn NvmeBDev>>>,
}

impl Rv6Syscalls {
    pub fn new(
        create_xv6usr: Arc<dyn CreateXv6Usr + Send + Sync>,
        fs: VFSPtr,
        net: Box<dyn Net + Send>,
        nvme: Box<dyn NvmeBDev>,
    ) -> Self {
        Self {
            create_xv6usr,
            fs,
            net: Arc::new(Mutex::new(net)),
            nvme: Arc::new(Mutex::new(nvme)),
        }
    }
}

impl Xv6 for Rv6Syscalls {
    fn clone(&self) -> Box<dyn Xv6> {
        box Self {
            create_xv6usr: self.create_xv6usr.clone(),
            fs: self.fs.clone(),
            net: self.net.clone(),
            nvme: self.nvme.clone(),
        }
    }

    fn as_net(&self) -> Box<dyn Net> {
        box Self {
            create_xv6usr: self.create_xv6usr.clone(),
            fs: self.fs.clone(),
            net: self.net.clone(),
            nvme: self.nvme.clone(),
        }
    }

    fn as_nvme(&self) -> Box<dyn NvmeBDev> {
        box Self {
            create_xv6usr: self.create_xv6usr.clone(),
            fs: self.fs.clone(),
            net: self.net.clone(),
            nvme: self.nvme.clone(),
        }
    }

    fn sys_spawn_thread(&self, name: &str, func: Box<dyn FnOnce() + Send>) -> Box<dyn Thread> {
        crate::thread::spawn_thread(self.fs.clone(), name, func)
    }

    fn sys_spawn_domain(
        &self,
        rv6: Box<dyn Xv6>,
        path: &str,
        args: &str,
        fds: [Option<usize>; NFILE],
    ) -> Result<Box<dyn Thread>> {
        // Load bin into memory
        println!("sys_spawn_domain {} {}", path, args);
        let fd = self.fs.sys_open(path, FileMode::READ)?;
        let size = self.fs.sys_fstat(fd)?.size; // fstat will filter out non INode files
        let mut blob = alloc::vec![0; size as usize];
        assert_eq!(self.fs.sys_read(fd, blob.as_mut_slice())?, size as usize);

        // Create a seperate copy of all the objects we want to pass to the new thread
        // and transfer the ownership over
        let fs_copy = self.fs.clone();
        let path_copy = path.to_owned();
        let create_copy = self.create_xv6usr.clone();
        let args_copy = args.to_owned();
        let rv6 = self.clone();
        let tmp_storage_id = fs_copy.sys_save_threadlocal(fds)?;
        Ok(self.sys_spawn_thread(
            path,
            Box::new(move || {
                fs_copy.sys_set_threadlocal(tmp_storage_id).unwrap();
                create_copy.create_domain_xv6usr(&path_copy, rv6, blob.as_slice(), &args_copy);
            }),
        ))
    }

    fn sys_rdtsc(&self) -> u64 {
        libtime::get_rdtsc()
    }
}

impl UsrVFS for Rv6Syscalls {
    fn sys_open(&self, path: &str, mode: FileMode) -> Result<usize> {
        self.fs.sys_open(path, mode)
    }
    fn sys_close(&self, fd: usize) -> Result<()> {
        self.fs.sys_close(fd)
    }
    fn sys_read(&self, fd: usize, buffer: &mut [u8]) -> Result<usize> {
        self.fs.sys_read(fd, buffer)
    }
    fn sys_write(&self, fd: usize, buffer: &[u8]) -> Result<usize> {
        self.fs.sys_write(fd, buffer)
    }
    fn sys_seek(&self, fd: usize, offset: usize) -> Result<()> {
        self.fs.sys_seek(fd, offset)
    }
    fn sys_fstat(&self, fd: usize) -> Result<FileStat> {
        self.fs.sys_fstat(fd)
    }
    fn sys_mknod(&self, path: &str, major: i16, minor: i16) -> Result<()> {
        self.fs.sys_mknod(path, major, minor)
    }
    fn sys_dup(&self, fd: usize) -> Result<usize> {
        self.fs.sys_dup(fd)
    }
    fn sys_pipe(&self) -> Result<(usize, usize)> {
        self.fs.sys_pipe()
    }
    fn sys_dump_inode(&self) {
        self.fs.sys_dump_inode()
    }
}

impl Net for Rv6Syscalls {
    fn submit_and_poll(
        &self,
        packets: &mut VecDeque<Vec<u8>>,
        reap_queue: &mut VecDeque<Vec<u8>>,
        tx: bool,
    ) -> RpcResult<Result<usize>> {
        self.net.lock().submit_and_poll(packets, reap_queue, tx)
    }

    fn poll(&self, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        self.net.lock().poll(collect, tx)
    }

    fn submit_and_poll_rref(
        &self,
        packets: RRefDeque<[u8; 1512], 32>,
        collect: RRefDeque<[u8; 1512], 32>,
        tx: bool,
        pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1512], 32>, RRefDeque<[u8; 1512], 32>)>> {
        self.net
            .lock()
            .submit_and_poll_rref(packets, collect, tx, pkt_len)
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<[u8; 1512], 512>,
        tx: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1512], 512>)>> {
        self.net.lock().poll_rref(collect, tx)
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        self.net.lock().get_stats()
    }
}

impl NvmeBDev for Rv6Syscalls {
    fn submit_and_poll_rref(
        &self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> Result<(usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)> {
        self.nvme
            .lock()
            .submit_and_poll_rref(submit, collect, write)
    }

    fn poll_rref(&mut self, collect: RRefDeque<BlkReq, 1024>) -> Result<(usize, RRefDeque<BlkReq, 1024>)> {
        self.nvme.lock().poll_rref(collect)
    }

    fn get_stats(&mut self) -> Result<(u64, u64)> {
        self.nvme.lock().get_stats()
    }
}
