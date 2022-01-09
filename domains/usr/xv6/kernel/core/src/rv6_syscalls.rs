use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::VecDeque;

use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use console::println;
use interface::bdev::{BlkReq, NvmeBDev};
use interface::domain_create::CreateRv6Usr;
use interface::net::{Net, NetworkStats};
use interface::rpc::RpcResult;
use interface::rref::{RRefDeque, RRefVec};
use interface::rv6::{Rv6, Thread};
use interface::tpm::UsrTpm;
use interface::usrnet::UsrNet;
use interface::vfs::{FileMode, FileStat, Result, UsrVFS, NFILE, VFS};

pub struct Rv6Syscalls {
    create_xv6usr: Arc<dyn CreateRv6Usr>,
    fs: Box<dyn VFS>,
    usrnet: Box<dyn UsrNet>,
    net: Box<dyn Net>,
    nvme: Arc<Mutex<Box<dyn NvmeBDev>>>,
    usrtpm: Box<dyn UsrTpm>,
    start_time: u64,
}

impl Rv6Syscalls {
    pub fn new(
        create_xv6usr: Arc<dyn CreateRv6Usr>,
        fs: Box<dyn VFS>,
        usrnet: Box<dyn UsrNet>,
        net: Box<dyn Net>,
        nvme: Box<dyn NvmeBDev>,
        usrtpm: Box<dyn UsrTpm>,
    ) -> Self {
        Self {
            create_xv6usr,
            fs,
            usrnet,
            net,
            nvme: Arc::new(Mutex::new(nvme)),
            usrtpm,
            start_time: libtime::get_ns_time(),
        }
    }

    fn _clone(&self) -> RpcResult<Self> {
        Ok(Self {
            start_time: self.start_time,
            create_xv6usr: self.create_xv6usr.clone(),
            fs: self.fs.clone()?,
            usrnet: self.usrnet.clone_usrnet()?,
            net: self.net.clone_net()?,
            nvme: self.nvme.clone(),
            usrtpm: self.usrtpm.clone_usrtpm()?,
        })
    }
}

impl Rv6 for Rv6Syscalls {
    fn clone_rv6(&self) -> RpcResult<Box<dyn Rv6>> {
        Ok(box self._clone()?)
    }

    fn as_vfs(&self) -> RpcResult<Box<dyn UsrVFS>> {
        Ok(box self._clone()?)
    }

    fn as_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        Ok(box self._clone()?)
    }

    fn get_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        self.usrnet.clone_usrnet()
    }

    fn get_usrtpm(&self) -> RpcResult<Box<dyn UsrTpm>> {
        self.usrtpm.clone_usrtpm()
    }

    fn as_net(&self) -> RpcResult<Box<dyn Net>> {
        Ok(box self._clone()?)
    }

    fn as_nvme(&self) -> RpcResult<Box<dyn NvmeBDev>> {
        Ok(box self._clone()?)
    }

    fn sys_spawn_thread(
        &self,
        name: RRefVec<u8>,
        func: Box<dyn FnOnce() + Send>,
    ) -> RpcResult<Result<Box<dyn Thread>>> {
        Ok((|| {
            let name = core::str::from_utf8(name.as_slice())?;
            Ok(crate::thread::spawn_thread(
                self.fs.clone().unwrap(),
                &name,
                func,
            ))
        })())
    }

    fn sys_spawn_domain(
        &self,
        rv6: Box<dyn Rv6>,
        path: RRefVec<u8>,
        args: RRefVec<u8>,
        fds: [Option<usize>; NFILE],
    ) -> RpcResult<Result<Box<dyn Thread>>> {
        Ok((|| {
            // Load bin into memory
            let path_slice = core::str::from_utf8(path.as_slice())?;
            let args_slice = core::str::from_utf8(args.as_slice())?;
            println!("sys_spawn_domain {} {}", path_slice, args_slice);
            let path_copy = path_slice.to_owned();
            let args_copy = args_slice.to_owned();

            let (fd, path) = self.fs.sys_open(path, FileMode::READ)??;
            let size = self.fs.sys_fstat(fd)??.size; // fstat will filter out non INode files
            let blob = RRefVec::new(0, size as usize);
            let (bytes_read, blob) = self.fs.sys_read(fd, blob)??;
            assert_eq!(bytes_read, size as usize);

            // Create a seperate copy of all the objects we want to pass to the new thread
            // and transfer the ownership over
            let fs_copy = self.fs.clone()?;
            let create_copy = self.create_xv6usr.clone();
            let tmp_storage_id = fs_copy.sys_save_threadlocal(fds)??;
            Ok(self
                .sys_spawn_thread(
                    path,
                    Box::new(move || {
                        fs_copy.sys_set_threadlocal(tmp_storage_id).unwrap();
                        create_copy.create_domain_xv6usr(
                            &path_copy,
                            blob.as_slice(),
                            rv6,
                            &args_copy,
                        );
                    }),
                )?
                .unwrap())
        })())
    }

    fn sys_getpid(&self) -> RpcResult<Result<u64>> {
        Ok({ Ok(libsyscalls::syscalls::sys_current_thread_id()) })
    }

    fn sys_uptime(&self) -> RpcResult<Result<u64>> {
        Ok({ Ok(libtime::get_ns_time() - self.start_time) })
    }

    fn sys_sleep(&self, ns: u64) -> RpcResult<Result<()>> {
        Ok({
            libtime::sys_ns_sleep(ns);
            Ok(())
        })
    }
}

impl UsrVFS for Rv6Syscalls {
    fn sys_open(
        &self,
        path: RRefVec<u8>,
        mode: FileMode,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.fs.sys_open(path, mode)
    }
    fn sys_close(&self, fd: usize) -> RpcResult<Result<()>> {
        self.fs.sys_close(fd)
    }
    fn sys_read(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.fs.sys_read(fd, buffer)
    }
    fn sys_write(&self, fd: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.fs.sys_write(fd, buffer)
    }
    fn sys_seek(&self, fd: usize, offset: usize) -> RpcResult<Result<()>> {
        self.fs.sys_seek(fd, offset)
    }
    fn sys_fstat(&self, fd: usize) -> RpcResult<Result<FileStat>> {
        self.fs.sys_fstat(fd)
    }
    fn sys_mknod(&self, path: RRefVec<u8>, major: i16, minor: i16) -> RpcResult<Result<()>> {
        self.fs.sys_mknod(path, major, minor)
    }
    fn sys_dup(&self, fd: usize) -> RpcResult<Result<usize>> {
        self.fs.sys_dup(fd)
    }
    fn sys_pipe(&self) -> RpcResult<Result<(usize, usize)>> {
        self.fs.sys_pipe()
    }
    fn sys_link(&self, old_path: RRefVec<u8>, new_path: RRefVec<u8>) -> RpcResult<Result<()>> {
        self.fs.sys_link(old_path, new_path)
    }
    fn sys_unlink(&self, path: RRefVec<u8>) -> RpcResult<Result<()>> {
        self.fs.sys_unlink(path)
    }
    fn sys_mkdir(&self, path: RRefVec<u8>) -> RpcResult<Result<()>> {
        self.fs.sys_mkdir(path)
    }
    fn sys_dump_inode(&self) -> RpcResult<Result<()>> {
        self.fs.sys_dump_inode()
    }
}

impl UsrNet for Rv6Syscalls {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        self.usrnet.clone_usrnet()
    }
    fn listen(&self, socket: usize, port: u16) -> RpcResult<Result<()>> {
        self.usrnet.listen(socket, port)
    }
    fn create(&self) -> RpcResult<Result<usize>> {
        self.usrnet.create()
    }
    fn poll(&self, tx: bool) -> RpcResult<Result<()>> {
        self.usrnet.poll(tx)
    }
    fn can_recv(&self, server: usize) -> RpcResult<Result<bool>> {
        self.usrnet.can_recv(server)
    }
    fn is_listening(&self, server: usize) -> RpcResult<Result<bool>> {
        self.usrnet.is_listening(server)
    }
    fn is_active(&self, server: usize) -> RpcResult<Result<bool>> {
        self.usrnet.is_active(server)
    }
    fn close(&self, server: usize) -> RpcResult<Result<()>> {
        self.usrnet.close(server)
    }
    fn read_socket(
        &self,
        socket: usize,
        buffer: RRefVec<u8>,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.usrnet.read_socket(socket, buffer)
    }
    fn write_socket(
        &self,
        socket: usize,
        buffer: RRefVec<u8>,
        size: usize,
    ) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        self.usrnet.write_socket(socket, buffer, size)
    }
}

impl Net for Rv6Syscalls {
    fn clone_net(&self) -> RpcResult<Box<dyn Net>> {
        self.net.clone_net()
    }
    fn submit_and_poll(
        &self,
        packets: &mut VecDeque<Vec<u8>>,
        reap_queue: &mut VecDeque<Vec<u8>>,
        tx: bool,
    ) -> RpcResult<Result<usize>> {
        self.net.submit_and_poll(packets, reap_queue, tx)
    }

    fn poll(&self, collect: &mut VecDeque<Vec<u8>>, tx: bool) -> RpcResult<Result<usize>> {
        self.net.poll(collect, tx)
    }

    fn submit_and_poll_rref(
        &self,
        packets: RRefDeque<[u8; 1514], 32>,
        collect: RRefDeque<[u8; 1514], 32>,
        tx: bool,
        pkt_len: usize,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>, RRefDeque<[u8; 1514], 32>)>> {
        self.net.submit_and_poll_rref(packets, collect, tx, pkt_len)
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<[u8; 1514], 512>,
        tx: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>> {
        self.net.poll_rref(collect, tx)
    }

    fn get_stats(&self) -> RpcResult<Result<NetworkStats>> {
        self.net.get_stats()
    }

    fn test_domain_crossing(&self) -> RpcResult<()> {
        self.net.test_domain_crossing()
    }
}

impl NvmeBDev for Rv6Syscalls {
    fn submit_and_poll_rref(
        &self,
        submit: RRefDeque<BlkReq, 128>,
        collect: RRefDeque<BlkReq, 128>,
        write: bool,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 128>, RRefDeque<BlkReq, 128>)>> {
        self.nvme
            .lock()
            .submit_and_poll_rref(submit, collect, write)
    }

    fn poll_rref(
        &self,
        collect: RRefDeque<BlkReq, 1024>,
    ) -> RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>> {
        self.nvme.lock().poll_rref(collect)
    }

    fn get_stats(&self) -> RpcResult<Result<(u64, u64)>> {
        self.nvme.lock().get_stats()
    }
}
