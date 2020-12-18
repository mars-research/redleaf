#![feature(prelude_import)]
#![no_std]
#![feature(trait_alias)]
#[prelude_import]
use core::prelude::v1::*;
#[macro_use]
extern crate core;
#[macro_use]
extern crate compiler_builtins;

extern crate alloc;
#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate codegen_proc;

pub mod bdev {
    /// RedLeaf block device interface
    use rref::{RRef, RRefDeque, traits::TypeIdentifiable};
    use crate::error::Result;
    use crate::rpc::RpcResult;
    pub const BSIZE: usize = 4096;
    pub trait BDev: Send + Sync {
        fn read(&self, block: u32, data: RRef<[u8; BSIZE]>)
        -> RpcResult<RRef<[u8; BSIZE]>>;
        fn write(&self, block: u32, data: &RRef<[u8; BSIZE]>)
        -> RpcResult<()>;
    }
    pub struct BlkReq {
        pub data: [u8; 4096],
        pub data_len: usize,
        pub block: u64,
    }
    impl TypeIdentifiable for BlkReq {
        fn type_id() -> u64 { 1 }
    }
    impl BlkReq {
        pub fn new() -> Self {
            Self{data: [0u8; 4096], data_len: 4096, block: 0,}
        }
        pub fn from_data(data: [u8; 4096]) -> Self {
            Self{data, data_len: 4096, block: 0,}
        }
    }
    pub trait NvmeBDev: Send {
        fn submit_and_poll_rref(&self, submit: RRefDeque<BlkReq, 128>,
                                collect: RRefDeque<BlkReq, 128>, write: bool)
        ->
            RpcResult<Result<(usize, RRefDeque<BlkReq, 128>,
                              RRefDeque<BlkReq, 128>)>>;
        fn poll_rref(&mut self, collect: RRefDeque<BlkReq, 1024>)
        -> RpcResult<Result<(usize, RRefDeque<BlkReq, 1024>)>>;
        fn get_stats(&mut self)
        -> RpcResult<Result<(u64, u64)>>;
    }
}
pub mod dom_a {
    /// RedLeaf block device interface
    use alloc::boxed::Box;
    use rref::{RRef, RRefDeque};
    pub trait DomA {
        fn ping_pong(&self, buffer: RRef<[u8; 1024]>)
        -> RRef<[u8; 1024]>;
        fn tx_submit_and_poll(&mut self, packets: RRefDeque<[u8; 100], 32>,
                              reap_queue: RRefDeque<[u8; 100], 32>)
        -> (usize, RRefDeque<[u8; 100], 32>, RRefDeque<[u8; 100], 32>);
    }
}
pub mod dom_c {
    use rref::RRef;
    use crate::rpc::RpcResult;
    pub trait DomC {
        fn no_arg(&self)
        -> RpcResult<()>;
        fn one_arg(&self, x: usize)
        -> RpcResult<usize>;
        fn one_rref(&self, x: RRef<usize>)
        -> RpcResult<RRef<usize>>;
    }
    struct DomCProxy {
        domain: ::alloc::boxed::Box<dyn DomC>,
        domain_id: u64,
    }
    unsafe impl Sync for DomCProxy { }
    unsafe impl Send for DomCProxy { }
    impl DomCProxy {
        fn new(domain_id: u64, domain: ::alloc::boxed::Box<dyn DomC>)
         -> Self {
            Self{domain, domain_id,}
        }
    }
    impl DomC for DomCProxy {
        fn no_arg(&self) -> RpcResult<()> {
            let caller_domain =
                unsafe { sys_update_current_domain_id(self.domain_id) };
            #[cfg(not(feature = "trampoline"))]
            let r = self.domain.no_arg(&self);
            unsafe { sys_update_current_domain_id(caller_domain) };
            r
        }
        fn one_arg(&self, x: usize) -> RpcResult<usize> {
            let caller_domain =
                unsafe { sys_update_current_domain_id(self.domain_id) };
            #[cfg(not(feature = "trampoline"))]
            let r = self.domain.one_arg(&self, x: usize);
            unsafe { sys_update_current_domain_id(caller_domain) };
            r
        }
        fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>> {
            let caller_domain =
                unsafe { sys_update_current_domain_id(self.domain_id) };
            #[cfg(not(feature = "trampoline"))]
            let r = self.domain.one_rref(&self, x: RRef<usize>);
            unsafe { sys_update_current_domain_id(caller_domain) };
            r
        }
    }
    #[no_mangle]
    extern fn one_arg(generated_proxy_domain_domc:
                          &alloc::boxed::Box<dyn DomC>, x: usize)
     -> RpcResult<usize> {
        generated_proxy_domain_domc.one_arg(x)
    }
    #[no_mangle]
    extern fn one_arg_err(generated_proxy_domain_domc:
                              &alloc::boxed::Box<dyn DomC>, x: usize)
     -> RpcResult<usize> {
        Err(unsafe { ::usr::rpc::RpcError::panic() })
    }
    #[no_mangle]
    extern "C" fn one_arg_addr() -> u64 { one_arg_err as u64 }
    extern {
        fn one_arg_tramp(generated_proxy_domain_domc:
                             &alloc::boxed::Box<dyn DomC>, x: usize)
        -> RpcResult<usize>;
    }
    #[no_mangle]
    extern fn one_rref(generated_proxy_domain_domc:
                           &alloc::boxed::Box<dyn DomC>, x: RRef<usize>)
     -> RpcResult<RRef<usize>> {
        generated_proxy_domain_domc.one_rref(x)
    }
    #[no_mangle]
    extern fn one_rref_err(generated_proxy_domain_domc:
                               &alloc::boxed::Box<dyn DomC>, x: RRef<usize>)
     -> RpcResult<RRef<usize>> {
        Err(unsafe { ::usr::rpc::RpcError::panic() })
    }
    #[no_mangle]
    extern "C" fn one_rref_addr() -> u64 { one_rref_err as u64 }
    extern {
        fn one_rref_tramp(generated_proxy_domain_domc:
                              &alloc::boxed::Box<dyn DomC>, x: RRef<usize>)
        -> RpcResult<RRef<usize>>;
    }
}
pub mod error {
    use alloc::boxed::Box;
    use core::fmt;
    use crate::rpc::RpcError;
    /// A specialized [`Result`](../result/enum.Result.html) type for I/O
    /// operations.
    ///
    /// This type is broadly used across [`std::io`] for any operation which may
    /// produce an error.
    ///
    /// This typedef is generally used to avoid writing out [`io::Error`] directly and
    /// is otherwise a direct mapping to [`Result`].
    ///
    /// While usual Rust style is to import types directly, aliases of [`Result`]
    /// often are not, to make it easier to distinguish between them. [`Result`] is
    /// generally assumed to be [`std::result::Result`][`Result`], and so users of this alias
    /// will generally use `io::Result` instead of shadowing the prelude's import
    /// of [`std::result::Result`][`Result`].
    ///
    /// [`std::io`]: ../io/index.html
    /// [`io::Error`]: ../io/struct.Error.html
    /// [`Result`]: ../result/enum.Result.html
    ///
    /// # Examples
    ///
    /// A convenience function that bubbles an `io::Result` to its caller:
    ///
    /// ```
    /// use std::io;
    ///
    /// fn get_string() -> io::Result<String> {
    ///     let mut buffer = String::new();
    ///
    ///     io::stdin().read_line(&mut buffer)?;
    ///
    ///     Ok(buffer)
    /// }
    /// ```
    pub type Result<T> = core::result::Result<T, ErrorKind>;
    /// A list specifying general categories of I/O error.
    ///
    /// This list is intended to grow over time and it is not recommended to
    /// exhaustively match against it.
    ///
    /// It is used with the [`io::Error`] type.
    ///
    /// [`io::Error`]: struct.Error.html
    #[allow(deprecated)]
    #[non_exhaustive]
    pub enum ErrorKind {

        /// The file was not found.
        FileNotFound,

        /// The operation lacked the necessary privileges to complete.
        PermissionDenied,

        /// The connection was refused by the remote server.
        ConnectionRefused,

        /// The connection was reset by the remote server.
        ConnectionReset,

        /// The connection was aborted (terminated) by the remote server.
        ConnectionAborted,

        /// The network operation failed because it was not connected yet.
        NotConnected,

        /// A socket address could not be bound because the address is already in
        /// use elsewhere.
        AddrInUse,

        /// A nonexistent interface was requested or the requested address was not
        /// local.
        AddrNotAvailable,

        /// The operation failed because a pipe was closed.
        BrokenPipe,

        /// The file already exists.
        FileAlreadyExists,

        /// The operation needs to block to complete, but the blocking operation was
        /// requested to not occur.
        WouldBlock,

        /// A parameter was incorrect.
        InvalidInput,

        /// Data not valid for the operation were encountered.
        ///
        /// Unlike [`InvalidInput`], this typically means that the operation
        /// parameters were valid, however the error was caused by malformed
        /// input data.
        ///
        /// For example, a function that reads a file into a string will error with
        /// `InvalidData` if the file's contents are not valid UTF-8.
        ///
        /// [`InvalidInput`]: #variant.InvalidInput
        InvalidData,

        /// The I/O operation's timeout expired, causing it to be canceled.
        TimedOut,

        /// An error returned when an operation could not be completed because a
        /// call to [`write`] returned [`Ok(0)`].
        ///
        /// This typically means that an operation could only succeed if it wrote a
        /// particular number of bytes but only a smaller number of bytes could be
        /// written.
        ///
        /// [`write`]: ../../std/io/trait.Write.html#tymethod.write
        /// [`Ok(0)`]: ../../std/io/type.Result.html
        WriteZero,

        /// This operation was interrupted.
        ///
        /// Interrupted operations can typically be retried.
        Interrupted,

        /// Any I/O error not part of this list.
        Other,

        /// An error returned when an operation could not be completed because an
        /// "end of file" was reached prematurely.
        ///
        /// This typically means that an operation could only succeed if it read a
        /// particular number of bytes but only a smaller number of bytes could be
        /// read.
        UnexpectedEof,

        /// Format error when write_fmtks
        FormatError,

        /// Too many open files
        TooManyOpenedFiles,

        /// Invalid cross-thread-temp-storage id
        InvalidCTTSId,

        /// Invalid file descriptor
        InvalidFileDescriptor,

        /// Invalid device major number
        InvalidMajor,

        /// Inode cache ran out of nodes
        ICacheExhausted,

        /// No more free inode that we can allocate
        OutOfINode,

        /// Invalid file type
        InvalidFileType,

        /// No more empty slot in the directory that we can allocate
        /// a file with
        DirectoryExhausted,

        /// Operation not supported, like seeking a non-inode file.
        UnsupportedOperation,

        /// Device not initialized
        UninitializedDevice,

        /// Rpc error, could be anything in `crate::rpc::ErrorEnum`
        RpcError,

        /// Utf8 conversion error
        Utf8Error,

        /// One or more parameter is invalid
        InvalidParameter,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::clone::Clone for ErrorKind {
        #[inline]
        fn clone(&self) -> ErrorKind { { *self } }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::marker::Copy for ErrorKind { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::fmt::Debug for ErrorKind {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&ErrorKind::FileNotFound,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("FileNotFound");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::PermissionDenied,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("PermissionDenied");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::ConnectionRefused,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("ConnectionRefused");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::ConnectionReset,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("ConnectionReset");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::ConnectionAborted,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("ConnectionAborted");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::NotConnected,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("NotConnected");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::AddrInUse,) => {
                    let mut debug_trait_builder = f.debug_tuple("AddrInUse");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::AddrNotAvailable,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("AddrNotAvailable");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::BrokenPipe,) => {
                    let mut debug_trait_builder = f.debug_tuple("BrokenPipe");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::FileAlreadyExists,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("FileAlreadyExists");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::WouldBlock,) => {
                    let mut debug_trait_builder = f.debug_tuple("WouldBlock");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidInput,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidInput");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidData,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidData");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::TimedOut,) => {
                    let mut debug_trait_builder = f.debug_tuple("TimedOut");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::WriteZero,) => {
                    let mut debug_trait_builder = f.debug_tuple("WriteZero");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::Interrupted,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("Interrupted");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::Other,) => {
                    let mut debug_trait_builder = f.debug_tuple("Other");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::UnexpectedEof,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("UnexpectedEof");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::FormatError,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("FormatError");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::TooManyOpenedFiles,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("TooManyOpenedFiles");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidCTTSId,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidCTTSId");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidFileDescriptor,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidFileDescriptor");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidMajor,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidMajor");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::ICacheExhausted,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("ICacheExhausted");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::OutOfINode,) => {
                    let mut debug_trait_builder = f.debug_tuple("OutOfINode");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidFileType,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidFileType");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::DirectoryExhausted,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("DirectoryExhausted");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::UnsupportedOperation,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("UnsupportedOperation");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::UninitializedDevice,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("UninitializedDevice");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::RpcError,) => {
                    let mut debug_trait_builder = f.debug_tuple("RpcError");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::Utf8Error,) => {
                    let mut debug_trait_builder = f.debug_tuple("Utf8Error");
                    debug_trait_builder.finish()
                }
                (&ErrorKind::InvalidParameter,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("InvalidParameter");
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[allow(deprecated)]
    impl ::core::marker::StructuralEq for ErrorKind { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::cmp::Eq for ErrorKind {
        #[inline]
        #[doc(hidden)]
        fn assert_receiver_is_total_eq(&self) -> () { { } }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::hash::Hash for ErrorKind {
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            match (&*self,) {
                _ => {
                    ::core::hash::Hash::hash(&::core::intrinsics::discriminant_value(self),
                                             state)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::cmp::Ord for ErrorKind {
        #[inline]
        fn cmp(&self, other: &ErrorKind) -> ::core::cmp::Ordering {
            {
                let __self_vi =
                    ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi =
                    ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ => ::core::cmp::Ordering::Equal,
                    }
                } else { __self_vi.cmp(&__arg_1_vi) }
            }
        }
    }
    #[allow(deprecated)]
    impl ::core::marker::StructuralPartialEq for ErrorKind { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::cmp::PartialEq for ErrorKind {
        #[inline]
        fn eq(&self, other: &ErrorKind) -> bool {
            {
                let __self_vi =
                    ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi =
                    ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) { _ => true, }
                } else { false }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    #[allow(deprecated)]
    impl ::core::cmp::PartialOrd for ErrorKind {
        #[inline]
        fn partial_cmp(&self, other: &ErrorKind)
         -> ::core::option::Option<::core::cmp::Ordering> {
            {
                let __self_vi =
                    ::core::intrinsics::discriminant_value(&*self);
                let __arg_1_vi =
                    ::core::intrinsics::discriminant_value(&*other);
                if true && __self_vi == __arg_1_vi {
                    match (&*self, &*other) {
                        _ =>
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal),
                    }
                } else { __self_vi.partial_cmp(&__arg_1_vi) }
            }
        }
    }
    impl core::convert::From<RpcError> for ErrorKind {
        fn from(_: RpcError) -> Self { Self::RpcError }
    }
    impl core::convert::From<core::str::Utf8Error> for ErrorKind {
        fn from(_: core::str::Utf8Error) -> Self { Self::Utf8Error }
    }
}
pub mod net {
    /// RedLeaf network interface
    use alloc::boxed::Box;
    use rref::{RRef, RRefDeque};
    use alloc::{vec::Vec, collections::VecDeque};
    use crate::error::Result;
    use crate::rpc::RpcResult;
    use core::fmt;
    pub struct NetworkStats {
        pub tx_count: u64,
        pub rx_count: u64,
        pub tx_dma_ok: u64,
        pub rx_dma_ok: u64,
        pub rx_missed: u64,
        pub rx_crc_err: u64,
    }
    impl NetworkStats {
        pub fn new() -> Self {
            Self{tx_count: 0,
                 rx_count: 0,
                 tx_dma_ok: 0,
                 rx_dma_ok: 0,
                 rx_missed: 0,
                 rx_crc_err: 0,}
        }
        pub fn stats_diff(&mut self, start: NetworkStats) {
            self.tx_count.saturating_sub(start.tx_count);
            self.rx_count.saturating_sub(start.rx_count);
            self.tx_dma_ok.saturating_sub(start.tx_dma_ok);
            self.rx_dma_ok.saturating_sub(start.rx_dma_ok);
            self.rx_missed.saturating_sub(start.rx_missed);
            self.rx_crc_err.saturating_sub(start.rx_crc_err);
        }
    }
    impl fmt::Display for NetworkStats {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_fmt(::core::fmt::Arguments::new_v1(&["=> Tx stats: Count: ",
                                                         " dma_OK: ", "\n"],
                                                       &match (&self.tx_count,
                                                               &self.tx_dma_ok)
                                                            {
                                                            (arg0, arg1) =>
                                                            [::core::fmt::ArgumentV1::new(arg0,
                                                                                          ::core::fmt::Display::fmt),
                                                             ::core::fmt::ArgumentV1::new(arg1,
                                                                                          ::core::fmt::Display::fmt)],
                                                        }));
            f.write_fmt(::core::fmt::Arguments::new_v1(&["=> Rx stats: Count: ",
                                                         " dma_OK: ",
                                                         " missed: ",
                                                         " crc_err: "],
                                                       &match (&self.rx_count,
                                                               &self.rx_dma_ok,
                                                               &self.rx_missed,
                                                               &self.rx_crc_err)
                                                            {
                                                            (arg0, arg1, arg2,
                                                             arg3) =>
                                                            [::core::fmt::ArgumentV1::new(arg0,
                                                                                          ::core::fmt::Display::fmt),
                                                             ::core::fmt::ArgumentV1::new(arg1,
                                                                                          ::core::fmt::Display::fmt),
                                                             ::core::fmt::ArgumentV1::new(arg2,
                                                                                          ::core::fmt::Display::fmt),
                                                             ::core::fmt::ArgumentV1::new(arg3,
                                                                                          ::core::fmt::Display::fmt)],
                                                        }))
        }
    }
    pub trait Net: Send + Sync {
        fn clone_net(&self)
        -> RpcResult<Box<dyn Net>>;
        fn submit_and_poll(&self, packets: &mut VecDeque<Vec<u8>>,
                           reap_queue: &mut VecDeque<Vec<u8>>, tx: bool)
        -> RpcResult<Result<usize>>;
        fn poll(&self, collect: &mut VecDeque<Vec<u8>>, tx: bool)
        -> RpcResult<Result<usize>>;
        fn submit_and_poll_rref(&self, packets: RRefDeque<[u8; 1514], 32>,
                                collect: RRefDeque<[u8; 1514], 32>, tx: bool,
                                pkt_len: usize)
        ->
            RpcResult<Result<(usize, RRefDeque<[u8; 1514], 32>,
                              RRefDeque<[u8; 1514], 32>)>>;
        fn poll_rref(&self, collect: RRefDeque<[u8; 1514], 512>, tx: bool)
        -> RpcResult<Result<(usize, RRefDeque<[u8; 1514], 512>)>>;
        fn get_stats(&self)
        -> RpcResult<Result<NetworkStats>>;
        fn test_domain_crossing(&self)
        -> RpcResult<()>;
    }
}
pub mod pci {
    /// RedLeaf PCI bus driver interface
    use alloc::boxed::Box;
    use rref::{RRef, RRefDeque};
    use pci_driver::{PciDriver, PciClass, BarRegions, PciDrivers};
    pub trait PCI: Send {
        fn pci_register_driver(&self,
                               pci_driver: &mut dyn pci_driver::PciDriver,
                               bar_index: usize,
                               class: Option<(PciClass, u8)>)
        -> Result<(), ()>;
        /// Boxed trait objects cannot be cloned trivially!
        /// https://users.rust-lang.org/t/solved-is-it-possible-to-clone-a-boxed-trait-object/1714/6
        fn pci_clone(&self)
        -> Box<dyn PCI>;
    }
    pub trait PciResource {
        fn read(&self, bus: u8, dev: u8, func: u8, offset: u8)
        -> u32;
        fn write(&self, bus: u8, dev: u8, func: u8, offset: u8, value: u32);
    }
    pub trait PciBar {
        fn get_bar_region(&self, base: u64, size: usize,
                          pci_driver: pci_driver::PciDrivers)
        -> pci_driver::BarRegions;
    }
}
pub mod rpc {
    /// `RpcResult` is a wrapper around the `Result` type. It forces the users
    /// can only return an `Ok` and an `RpcError` must be raise by the proxy(trusted)
    use crate::error::ErrorKind;
    pub type RpcResult<T> = Result<T, RpcError>;
    /// A wrapper that hides the ErrorEnum
    pub struct RpcError {
        error: ErrorEnum,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for RpcError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                RpcError { error: ref __self_0_0 } => {
                    let mut debug_trait_builder = f.debug_struct("RpcError");
                    let _ =
                        debug_trait_builder.field("error", &&(*__self_0_0));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for RpcError { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for RpcError {
        #[inline]
        fn clone(&self) -> RpcError {
            { let _: ::core::clone::AssertParamIsClone<ErrorEnum>; *self }
        }
    }
    impl RpcError {
        pub unsafe fn panic() -> Self { Self{error: ErrorEnum::PanicUnwind,} }
    }
    enum ErrorEnum {

        /// Callee domain is panicked and unwinded
        PanicUnwind,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for ErrorEnum {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&ErrorEnum::PanicUnwind,) => {
                    let mut debug_trait_builder =
                        f.debug_tuple("PanicUnwind");
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for ErrorEnum { }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for ErrorEnum {
        #[inline]
        fn clone(&self) -> ErrorEnum { { *self } }
    }
}
pub mod usrnet {
    use alloc::boxed::Box;
    use rref::RRefVec;
    use crate::rpc::RpcResult;
    use crate::error::Result;
    /// UsrNet interface
    pub trait UsrNet: Send + Sync {
        fn clone_usrnet(&self)
        -> RpcResult<Box<dyn UsrNet>>;
        fn create(&self)
        -> RpcResult<Result<usize>>;
        fn listen(&self, socket: usize, port: u16)
        -> RpcResult<Result<()>>;
        fn poll(&self, tx: bool)
        -> RpcResult<Result<()>>;
        fn can_recv(&self, server: usize)
        -> RpcResult<Result<bool>>;
        fn is_listening(&self, server: usize)
        -> RpcResult<Result<bool>>;
        fn is_active(&self, socket: usize)
        -> RpcResult<Result<bool>>;
        fn close(&self, server: usize)
        -> RpcResult<Result<()>>;
        fn read_socket(&self, socket: usize, buffer: RRefVec<u8>)
        -> RpcResult<Result<(usize, RRefVec<u8>)>>;
        fn write_socket(&self, socket: usize, buffer: RRefVec<u8>,
                        size: usize)
        -> RpcResult<Result<(usize, RRefVec<u8>)>>;
    }
}
pub mod vfs {
    /// Virtual file system interface
    /// Implemented by xv6 file system
    /// Some of the syscalls do no return the buffer back to the caller. Feel free
    /// to change it if it's needed.
    use alloc::boxed::Box;
    use rref::RRefVec;
    pub use crate::vfs::file::{FileMode, FileStat, INodeFileType};
    pub use crate::vfs::directory::{DirectoryEntry, DirectoryEntryRef};
    pub use crate::error::{Result, ErrorKind};
    use crate::rpc::RpcResult;
    pub mod file {
        pub struct FileMode {
            bits: u32,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for FileMode { }
        impl ::core::marker::StructuralPartialEq for FileMode { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for FileMode {
            #[inline]
            fn eq(&self, other: &FileMode) -> bool {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        (*__self_0_0) == (*__self_1_0),
                    },
                }
            }
            #[inline]
            fn ne(&self, other: &FileMode) -> bool {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        (*__self_0_0) != (*__self_1_0),
                    },
                }
            }
        }
        impl ::core::marker::StructuralEq for FileMode { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Eq for FileMode {
            #[inline]
            #[doc(hidden)]
            fn assert_receiver_is_total_eq(&self) -> () {
                { let _: ::core::cmp::AssertParamIsEq<u32>; }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for FileMode {
            #[inline]
            fn clone(&self) -> FileMode {
                { let _: ::core::clone::AssertParamIsClone<u32>; *self }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialOrd for FileMode {
            #[inline]
            fn partial_cmp(&self, other: &FileMode)
             -> ::core::option::Option<::core::cmp::Ordering> {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        match ::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                   &(*__self_1_0))
                            {
                            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
                            =>
                            ::core::option::Option::Some(::core::cmp::Ordering::Equal),
                            cmp => cmp,
                        },
                    },
                }
            }
            #[inline]
            fn lt(&self, other: &FileMode) -> bool {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Greater)
                            == ::core::cmp::Ordering::Less,
                    },
                }
            }
            #[inline]
            fn le(&self, other: &FileMode) -> bool {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Greater)
                            != ::core::cmp::Ordering::Greater,
                    },
                }
            }
            #[inline]
            fn gt(&self, other: &FileMode) -> bool {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Less)
                            == ::core::cmp::Ordering::Greater,
                    },
                }
            }
            #[inline]
            fn ge(&self, other: &FileMode) -> bool {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        ::core::option::Option::unwrap_or(::core::cmp::PartialOrd::partial_cmp(&(*__self_0_0),
                                                                                               &(*__self_1_0)),
                                                          ::core::cmp::Ordering::Less)
                            != ::core::cmp::Ordering::Less,
                    },
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::Ord for FileMode {
            #[inline]
            fn cmp(&self, other: &FileMode) -> ::core::cmp::Ordering {
                match *other {
                    FileMode { bits: ref __self_1_0 } =>
                    match *self {
                        FileMode { bits: ref __self_0_0 } =>
                        match ::core::cmp::Ord::cmp(&(*__self_0_0),
                                                    &(*__self_1_0)) {
                            ::core::cmp::Ordering::Equal =>
                            ::core::cmp::Ordering::Equal,
                            cmp => cmp,
                        },
                    },
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::hash::Hash for FileMode {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                match *self {
                    FileMode { bits: ref __self_0_0 } => {
                        ::core::hash::Hash::hash(&(*__self_0_0), state)
                    }
                }
            }
        }
        impl ::bitflags::_core::fmt::Debug for FileMode {
            fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter)
             -> ::bitflags::_core::fmt::Result {
                #[allow(non_snake_case)]
                trait __BitFlags {
                    #[inline]
                    fn READ(&self) -> bool { false }
                    #[inline]
                    fn WRITE(&self) -> bool { false }
                    #[inline]
                    fn CREATE(&self) -> bool { false }
                    #[inline]
                    fn READWRITE(&self) -> bool { false }
                }
                impl __BitFlags for FileMode {
                    #[allow(deprecated)]
                    #[inline]
                    fn READ(&self) -> bool {
                        if Self::READ.bits == 0 && self.bits != 0 {
                            false
                        } else {
                            self.bits & Self::READ.bits == Self::READ.bits
                        }
                    }
                    #[allow(deprecated)]
                    #[inline]
                    fn WRITE(&self) -> bool {
                        if Self::WRITE.bits == 0 && self.bits != 0 {
                            false
                        } else {
                            self.bits & Self::WRITE.bits == Self::WRITE.bits
                        }
                    }
                    #[allow(deprecated)]
                    #[inline]
                    fn CREATE(&self) -> bool {
                        if Self::CREATE.bits == 0 && self.bits != 0 {
                            false
                        } else {
                            self.bits & Self::CREATE.bits == Self::CREATE.bits
                        }
                    }
                    #[allow(deprecated)]
                    #[inline]
                    fn READWRITE(&self) -> bool {
                        if Self::READWRITE.bits == 0 && self.bits != 0 {
                            false
                        } else {
                            self.bits & Self::READWRITE.bits ==
                                Self::READWRITE.bits
                        }
                    }
                }
                let mut first = true;
                if <FileMode as __BitFlags>::READ(self) {
                    if !first { f.write_str(" | ")?; }
                    first = false;
                    f.write_str("READ")?;
                }
                if <FileMode as __BitFlags>::WRITE(self) {
                    if !first { f.write_str(" | ")?; }
                    first = false;
                    f.write_str("WRITE")?;
                }
                if <FileMode as __BitFlags>::CREATE(self) {
                    if !first { f.write_str(" | ")?; }
                    first = false;
                    f.write_str("CREATE")?;
                }
                if <FileMode as __BitFlags>::READWRITE(self) {
                    if !first { f.write_str(" | ")?; }
                    first = false;
                    f.write_str("READWRITE")?;
                }
                let extra_bits = self.bits & !FileMode::all().bits();
                if extra_bits != 0 {
                    if !first { f.write_str(" | ")?; }
                    first = false;
                    f.write_str("0x")?;
                    ::bitflags::_core::fmt::LowerHex::fmt(&extra_bits, f)?;
                }
                if first { f.write_str("(empty)")?; }
                Ok(())
            }
        }
        impl ::bitflags::_core::fmt::Binary for FileMode {
            fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter)
             -> ::bitflags::_core::fmt::Result {
                ::bitflags::_core::fmt::Binary::fmt(&self.bits, f)
            }
        }
        impl ::bitflags::_core::fmt::Octal for FileMode {
            fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter)
             -> ::bitflags::_core::fmt::Result {
                ::bitflags::_core::fmt::Octal::fmt(&self.bits, f)
            }
        }
        impl ::bitflags::_core::fmt::LowerHex for FileMode {
            fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter)
             -> ::bitflags::_core::fmt::Result {
                ::bitflags::_core::fmt::LowerHex::fmt(&self.bits, f)
            }
        }
        impl ::bitflags::_core::fmt::UpperHex for FileMode {
            fn fmt(&self, f: &mut ::bitflags::_core::fmt::Formatter)
             -> ::bitflags::_core::fmt::Result {
                ::bitflags::_core::fmt::UpperHex::fmt(&self.bits, f)
            }
        }
        #[allow(dead_code)]
        impl FileMode {
            pub const READ: FileMode = FileMode{bits: 0b001,};
            pub const WRITE: FileMode = FileMode{bits: 0b010,};
            pub const CREATE: FileMode = FileMode{bits: 0b100,};
            pub const READWRITE: FileMode =
                FileMode{bits: Self::READ.bits | Self::WRITE.bits,};
            #[doc = r" Returns an empty set of flags"]
            #[inline]
            pub const fn empty() -> FileMode { FileMode{bits: 0,} }
            #[doc = r" Returns the set containing all flags."]
            #[inline]
            pub const fn all() -> FileMode {
                #[allow(non_snake_case)]
                trait __BitFlags {
                    const READ: u32 = 0;
                    const WRITE: u32 = 0;
                    const CREATE: u32 = 0;
                    const READWRITE: u32 = 0;
                }
                impl __BitFlags for FileMode {
                    #[allow(deprecated)]
                    const READ: u32 = Self::READ.bits;
                    #[allow(deprecated)]
                    const WRITE: u32 = Self::WRITE.bits;
                    #[allow(deprecated)]
                    const CREATE: u32 = Self::CREATE.bits;
                    #[allow(deprecated)]
                    const READWRITE: u32 = Self::READWRITE.bits;
                }
                FileMode{bits:
                             <FileMode as __BitFlags>::READ |
                                 <FileMode as __BitFlags>::WRITE |
                                 <FileMode as __BitFlags>::CREATE |
                                 <FileMode as __BitFlags>::READWRITE,}
            }
            #[doc = r" Returns the raw value of the flags currently stored."]
            #[inline]
            pub const fn bits(&self) -> u32 { self.bits }
            /// Convert from underlying bit representation, unless that
            /// representation contains bits that do not correspond to a flag.
            #[inline]
            pub fn from_bits(bits: u32)
             -> ::bitflags::_core::option::Option<FileMode> {
                if (bits & !FileMode::all().bits()) == 0 {
                    ::bitflags::_core::option::Option::Some(FileMode{bits,})
                } else { ::bitflags::_core::option::Option::None }
            }
            #[doc =
              r" Convert from underlying bit representation, dropping any bits"]
            #[doc = r" that do not correspond to flags."]
            #[inline]
            pub const fn from_bits_truncate(bits: u32) -> FileMode {
                FileMode{bits: bits & FileMode::all().bits,}
            }
            #[doc =
              r" Convert from underlying bit representation, preserving all"]
            #[doc =
              r" bits (even those not corresponding to a defined flag)."]
            #[inline]
            pub const unsafe fn from_bits_unchecked(bits: u32) -> FileMode {
                FileMode{bits,}
            }
            #[doc = r" Returns `true` if no flags are currently stored."]
            #[inline]
            pub const fn is_empty(&self) -> bool {
                self.bits() == FileMode::empty().bits()
            }
            #[doc = r" Returns `true` if all flags are currently set."]
            #[inline]
            pub const fn is_all(&self) -> bool {
                self.bits == FileMode::all().bits
            }
            #[doc =
              r" Returns `true` if there are flags common to both `self` and `other`."]
            #[inline]
            pub const fn intersects(&self, other: FileMode) -> bool {
                !FileMode{bits: self.bits & other.bits,}.is_empty()
            }
            #[doc =
              r" Returns `true` all of the flags in `other` are contained within `self`."]
            #[inline]
            pub const fn contains(&self, other: FileMode) -> bool {
                (self.bits & other.bits) == other.bits
            }
            /// Inserts the specified flags in-place.
            #[inline]
            pub fn insert(&mut self, other: FileMode) {
                self.bits |= other.bits;
            }
            /// Removes the specified flags in-place.
            #[inline]
            pub fn remove(&mut self, other: FileMode) {
                self.bits &= !other.bits;
            }
            /// Toggles the specified flags in-place.
            #[inline]
            pub fn toggle(&mut self, other: FileMode) {
                self.bits ^= other.bits;
            }
            /// Inserts or removes the specified flags depending on the passed value.
            #[inline]
            pub fn set(&mut self, other: FileMode, value: bool) {
                if value { self.insert(other); } else { self.remove(other); }
            }
        }
        impl ::bitflags::_core::ops::BitOr for FileMode {
            type Output = FileMode;
            /// Returns the union of the two sets of flags.
            #[inline]
            fn bitor(self, other: FileMode) -> FileMode {
                FileMode{bits: self.bits | other.bits,}
            }
        }
        impl ::bitflags::_core::ops::BitOrAssign for FileMode {
            /// Adds the set of flags.
            #[inline]
            fn bitor_assign(&mut self, other: FileMode) {
                self.bits |= other.bits;
            }
        }
        impl ::bitflags::_core::ops::BitXor for FileMode {
            type Output = FileMode;
            /// Returns the left flags, but with all the right flags toggled.
            #[inline]
            fn bitxor(self, other: FileMode) -> FileMode {
                FileMode{bits: self.bits ^ other.bits,}
            }
        }
        impl ::bitflags::_core::ops::BitXorAssign for FileMode {
            /// Toggles the set of flags.
            #[inline]
            fn bitxor_assign(&mut self, other: FileMode) {
                self.bits ^= other.bits;
            }
        }
        impl ::bitflags::_core::ops::BitAnd for FileMode {
            type Output = FileMode;
            /// Returns the intersection between the two sets of flags.
            #[inline]
            fn bitand(self, other: FileMode) -> FileMode {
                FileMode{bits: self.bits & other.bits,}
            }
        }
        impl ::bitflags::_core::ops::BitAndAssign for FileMode {
            /// Disables all flags disabled in the set.
            #[inline]
            fn bitand_assign(&mut self, other: FileMode) {
                self.bits &= other.bits;
            }
        }
        impl ::bitflags::_core::ops::Sub for FileMode {
            type Output = FileMode;
            /// Returns the set difference of the two sets of flags.
            #[inline]
            fn sub(self, other: FileMode) -> FileMode {
                FileMode{bits: self.bits & !other.bits,}
            }
        }
        impl ::bitflags::_core::ops::SubAssign for FileMode {
            /// Disables all flags enabled in the set.
            #[inline]
            fn sub_assign(&mut self, other: FileMode) {
                self.bits &= !other.bits;
            }
        }
        impl ::bitflags::_core::ops::Not for FileMode {
            type Output = FileMode;
            /// Returns the complement of this set of flags.
            #[inline]
            fn not(self) -> FileMode {
                FileMode{bits: !self.bits,} & FileMode::all()
            }
        }
        impl ::bitflags::_core::iter::Extend<FileMode> for FileMode {
            fn extend<T: ::bitflags::_core::iter::IntoIterator<Item =
                                                               FileMode>>(&mut self,
                                                                          iterator:
                                                                              T) {
                for item in iterator { self.insert(item) }
            }
        }
        impl ::bitflags::_core::iter::FromIterator<FileMode> for FileMode {
            fn from_iter<T: ::bitflags::_core::iter::IntoIterator<Item =
                                                                  FileMode>>(iterator:
                                                                                 T)
             -> FileMode {
                let mut result = Self::empty();
                result.extend(iterator);
                result
            }
        }
        pub struct FileStat {
            pub device: u32,
            pub inum: u16,
            pub file_type: INodeFileType,
            pub nlink: i16,
            pub size: u64,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for FileStat { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for FileStat {
            #[inline]
            fn clone(&self) -> FileStat {
                {
                    let _: ::core::clone::AssertParamIsClone<u32>;
                    let _: ::core::clone::AssertParamIsClone<u16>;
                    let _: ::core::clone::AssertParamIsClone<INodeFileType>;
                    let _: ::core::clone::AssertParamIsClone<i16>;
                    let _: ::core::clone::AssertParamIsClone<u64>;
                    *self
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for FileStat {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    FileStat {
                    device: ref __self_0_0,
                    inum: ref __self_0_1,
                    file_type: ref __self_0_2,
                    nlink: ref __self_0_3,
                    size: ref __self_0_4 } => {
                        let mut debug_trait_builder =
                            f.debug_struct("FileStat");
                        let _ =
                            debug_trait_builder.field("device",
                                                      &&(*__self_0_0));
                        let _ =
                            debug_trait_builder.field("inum",
                                                      &&(*__self_0_1));
                        let _ =
                            debug_trait_builder.field("file_type",
                                                      &&(*__self_0_2));
                        let _ =
                            debug_trait_builder.field("nlink",
                                                      &&(*__self_0_3));
                        let _ =
                            debug_trait_builder.field("size",
                                                      &&(*__self_0_4));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        #[repr(u16)]
        pub enum INodeFileType { Unitialized, Directory, File, Device, }
        impl ::core::marker::StructuralPartialEq for INodeFileType { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::cmp::PartialEq for INodeFileType {
            #[inline]
            fn eq(&self, other: &INodeFileType) -> bool {
                {
                    let __self_vi =
                        ::core::intrinsics::discriminant_value(&*self);
                    let __arg_1_vi =
                        ::core::intrinsics::discriminant_value(&*other);
                    if true && __self_vi == __arg_1_vi {
                        match (&*self, &*other) { _ => true, }
                    } else { false }
                }
            }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for INodeFileType { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for INodeFileType {
            #[inline]
            fn clone(&self) -> INodeFileType { { *self } }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for INodeFileType {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&INodeFileType::Unitialized,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Unitialized");
                        debug_trait_builder.finish()
                    }
                    (&INodeFileType::Directory,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("Directory");
                        debug_trait_builder.finish()
                    }
                    (&INodeFileType::File,) => {
                        let mut debug_trait_builder = f.debug_tuple("File");
                        debug_trait_builder.finish()
                    }
                    (&INodeFileType::Device,) => {
                        let mut debug_trait_builder = f.debug_tuple("Device");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        #[allow(non_upper_case_globals, unused_qualifications)]
        const _IMPL_NUM_FromPrimitive_FOR_INodeFileType: () =
            {
                #[allow(clippy :: useless_attribute)]
                #[allow(rust_2018_idioms)]
                extern crate num_traits as _num_traits;
                impl _num_traits::FromPrimitive for INodeFileType {
                    #[allow(trivial_numeric_casts)]
                    #[inline]
                    fn from_i64(n: i64) -> Option<Self> {
                        if n == INodeFileType::Unitialized as i64 {
                            Some(INodeFileType::Unitialized)
                        } else if n == INodeFileType::Directory as i64 {
                            Some(INodeFileType::Directory)
                        } else if n == INodeFileType::File as i64 {
                            Some(INodeFileType::File)
                        } else if n == INodeFileType::Device as i64 {
                            Some(INodeFileType::Device)
                        } else { None }
                    }
                    #[inline]
                    fn from_u64(n: u64) -> Option<Self> {
                        Self::from_i64(n as i64)
                    }
                }
            };
    }
    pub mod directory {
        use byteorder::{ByteOrder, LittleEndian};
        use core::convert::TryFrom;
        /// Max size of directory name
        const DIRSIZ: usize = 14;
        #[repr(C)]
        pub struct DirectoryEntry {
            pub inum: u16,
            pub name: [u8; DIRSIZ],
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for DirectoryEntry {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    DirectoryEntry {
                    inum: ref __self_0_0, name: ref __self_0_1 } => {
                        let mut debug_trait_builder =
                            f.debug_struct("DirectoryEntry");
                        let _ =
                            debug_trait_builder.field("inum",
                                                      &&(*__self_0_0));
                        let _ =
                            debug_trait_builder.field("name",
                                                      &&(*__self_0_1));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl DirectoryEntry {
            pub fn from_bytes(arr: &[u8]) -> Self {
                Self{inum: LittleEndian::read_u16(arr),
                     name:
                         <[u8; DIRSIZ]>::try_from(&arr[2..2 +
                                                              DIRSIZ]).unwrap(),}
            }
        }
        pub struct DirectoryEntryRef<'a> {
            pub inum: u16,
            pub name: &'a [u8],
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl <'a> ::core::fmt::Debug for DirectoryEntryRef<'a> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match *self {
                    DirectoryEntryRef {
                    inum: ref __self_0_0, name: ref __self_0_1 } => {
                        let mut debug_trait_builder =
                            f.debug_struct("DirectoryEntryRef");
                        let _ =
                            debug_trait_builder.field("inum",
                                                      &&(*__self_0_0));
                        let _ =
                            debug_trait_builder.field("name",
                                                      &&(*__self_0_1));
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        impl <'a> DirectoryEntryRef<'a> {
            pub fn from_bytes(arr: &'a [u8]) -> Self {
                Self{inum: LittleEndian::read_u16(&arr[..2]),
                     name: &arr[2..2 + DIRSIZ],}
            }
            pub fn as_bytes(&self)
             -> [u8; core::mem::size_of::<DirectoryEntry>()] {
                let mut bytes = [0u8; core::mem::size_of::<DirectoryEntry>()];
                LittleEndian::write_u16(&mut bytes[0..2], self.inum);
                bytes[2..].copy_from_slice(self.name);
                bytes
            }
        }
    }
    pub const NFILE: usize = 100;
    pub trait UsrVFS: Send + Sync {
        fn sys_open(&self, path: RRefVec<u8>, mode: FileMode)
        -> RpcResult<Result<(usize, RRefVec<u8>)>>;
        fn sys_close(&self, fd: usize)
        -> RpcResult<Result<()>>;
        fn sys_read(&self, fd: usize, buffer: RRefVec<u8>)
        -> RpcResult<Result<(usize, RRefVec<u8>)>>;
        fn sys_write(&self, fd: usize, buffer: RRefVec<u8>)
        -> RpcResult<Result<(usize, RRefVec<u8>)>>;
        fn sys_seek(&self, fd: usize, offset: usize)
        -> RpcResult<Result<()>>;
        fn sys_fstat(&self, fd: usize)
        -> RpcResult<Result<FileStat>>;
        fn sys_mknod(&self, path: RRefVec<u8>, major: i16, minor: i16)
        -> RpcResult<Result<()>>;
        fn sys_dup(&self, fd: usize)
        -> RpcResult<Result<usize>>;
        fn sys_pipe(&self)
        -> RpcResult<Result<(usize, usize)>>;
        fn sys_link(&self, old_path: RRefVec<u8>, new_path: RRefVec<u8>)
        -> RpcResult<Result<()>>;
        fn sys_unlink(&self, path: RRefVec<u8>)
        -> RpcResult<Result<()>>;
        fn sys_mkdir(&self, path: RRefVec<u8>)
        -> RpcResult<Result<()>>;
        fn sys_dump_inode(&self)
        -> RpcResult<Result<()>>;
    }
    pub trait KernelVFS: Send + Sync {
        fn sys_save_threadlocal(&self, fds: [Option<usize>; NFILE])
        -> Result<usize>;
        fn sys_set_threadlocal(&self, id: usize)
        -> Result<()>;
        fn sys_thread_exit(&self);
    }
    pub trait VFS: UsrVFS + KernelVFS + Send + Sync {
        fn clone(&self)
        -> Box<dyn VFS>;
    }
}
pub mod rv6 {
    /// Rv6 system calls
    use alloc::boxed::Box;
    use rref::RRefVec;
    use crate::vfs::{UsrVFS, NFILE};
    use crate::net::Net;
    use crate::usrnet::UsrNet;
    use crate::bdev::NvmeBDev;
    use crate::rpc::RpcResult;
    use crate::tpm::UsrTpm;
    pub use crate::vfs::{FileMode, FileStat};
    pub use crate::error::{ErrorKind, Result};
    pub trait Rv6: Send + Sync + UsrVFS + Net + UsrNet {
        fn clone(&self)
        -> RpcResult<Box<dyn Rv6>>;
        fn as_net(&self)
        -> RpcResult<Box<dyn Net>>;
        fn as_nvme(&self)
        -> RpcResult<Box<dyn NvmeBDev>>;
        fn as_usrnet(&self)
        -> RpcResult<Box<dyn UsrNet>>;
        fn get_usrnet(&self)
        -> RpcResult<Box<dyn UsrNet>>;
        fn get_usrtpm(&self)
        -> RpcResult<Box<dyn UsrTpm>>;
        fn sys_spawn_thread(&self, name: RRefVec<u8>,
                            func: alloc::boxed::Box<dyn FnOnce() + Send>)
        -> RpcResult<Result<Box<dyn Thread>>>;
        fn sys_spawn_domain(&self, rv6: Box<dyn Rv6>, path: RRefVec<u8>,
                            args: RRefVec<u8>, fds: [Option<usize>; NFILE])
        -> RpcResult<Result<Box<dyn Thread>>>;
        fn sys_getpid(&self)
        -> RpcResult<Result<u64>>;
        fn sys_uptime(&self)
        -> RpcResult<Result<u64>>;
        fn sys_sleep(&self, ns: u64)
        -> RpcResult<Result<()>>;
    }
    pub trait File: Send {
        fn read(&self, data: &mut [u8])
        -> usize;
        fn write(&self, data: &[u8])
        -> usize;
    }
    pub trait Thread: Send {
        fn join(&self)
        -> RpcResult<()>;
    }
}
pub mod tpm {
    mod tpm_dev {
        use alloc::boxed::Box;
        pub enum TpmRegs {
            TPM_ACCESS = 0x0000,
            TPM_INT_ENABLE = 0x0008,
            TPM_INT_VECTOR = 0x000C,
            TPM_INT_STATS = 0x0010,
            TPM_INTF_CAPABILITY = 0x0014,
            TPM_STS = 0x0018,
            TPM_DATA_FIFO = 0x0024,
            TPM_xDATA_FIFO = 0x0083,
            TPM_DID_VID = 0x0F00,
            TPM_RID = 0x0F04,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for TpmRegs { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for TpmRegs {
            #[inline]
            fn clone(&self) -> TpmRegs { { *self } }
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::fmt::Debug for TpmRegs {
            fn fmt(&self, f: &mut ::core::fmt::Formatter)
             -> ::core::fmt::Result {
                match (&*self,) {
                    (&TpmRegs::TPM_ACCESS,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_ACCESS");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_INT_ENABLE,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_INT_ENABLE");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_INT_VECTOR,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_INT_VECTOR");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_INT_STATS,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_INT_STATS");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_INTF_CAPABILITY,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_INTF_CAPABILITY");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_STS,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_STS");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_DATA_FIFO,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_DATA_FIFO");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_xDATA_FIFO,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_xDATA_FIFO");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_DID_VID,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_DID_VID");
                        debug_trait_builder.finish()
                    }
                    (&TpmRegs::TPM_RID,) => {
                        let mut debug_trait_builder =
                            f.debug_tuple("TPM_RID");
                        debug_trait_builder.finish()
                    }
                }
            }
        }
        pub trait TpmDev: Send + Sync {
            fn clone_tpmdev(&self)
            -> Box<dyn TpmDev>;
            fn read_u8(&self, locality: u32, reg: TpmRegs)
            -> u8;
            fn write_u8(&self, locality: u32, reg: TpmRegs, val: u8);
            fn read_u32(&self, locality: u32, reg: TpmRegs)
            -> u32;
            fn write_u32(&self, locality: u32, reg: TpmRegs, val: u32);
        }
    }
    mod usr_tpm {
        use alloc::vec::Vec;
        use alloc::boxed::Box;
        use core::mem;
        #[repr(packed)]
        pub struct TpmBankInfo {
            pub alg_id: u16,
            pub digest_size: u16,
            pub crypto_id: u16,
        }
        impl TpmBankInfo {
            pub fn new(alg_id: u16, digest_size: u16, crypto_id: u16)
             -> Self {
                Self{alg_id: alg_id.swap_bytes().to_be(),
                     digest_size: digest_size.swap_bytes().to_be(),
                     crypto_id: crypto_id.swap_bytes().to_be(),}
            }
        }
        pub struct TpmDevInfo {
            pub nr_allocated_banks: u32,
            pub allocated_banks: Vec<TpmBankInfo>,
        }
        impl TpmDevInfo {
            pub fn new(nr_allocated_banks: u32,
                       allocated_banks: Vec<TpmBankInfo>) -> Self {
                Self{nr_allocated_banks: nr_allocated_banks,
                     allocated_banks: allocated_banks,}
            }
        }
        pub struct TpmTHa {
            pub hash_alg: u16,
            pub digest: Vec<u8>,
        }
        impl TpmTHa {
            pub fn new(hash_alg: u16, digest: Vec<u8>) -> Self {
                Self{hash_alg: hash_alg, digest: digest,}
            }
            pub fn size(&self) -> usize {
                let ret: usize =
                    mem::size_of::<u16>() +
                        self.digest.len() * mem::size_of::<u8>();
                ret
            }
            pub fn to_vec(&self) -> Vec<u8> {
                let mut buf: Vec<u8> = Vec::with_capacity(self.size());
                buf.extend_from_slice(&u16::to_be_bytes(self.hash_alg));
                buf.extend_from_slice(&self.digest);
                buf
            }
        }
        pub enum TpmAlgorithms {
            TPM_ALG_ERROR = 0x0000,
            TPM_ALG_RSA = 0x0001,
            TPM_ALG_SHA1 = 0x0004,
            TPM_ALG_HMAC = 0x0005,
            TPM_ALG_AES = 0x0006,
            TPM_ALG_KEYEDHASH = 0x0008,
            TPM_ALG_XOR = 0x000A,
            TPM_ALG_SHA256 = 0x000B,
            TPM_ALG_SHA384 = 0x000C,
            TPM_ALG_SHA512 = 0x000D,
            TPM_ALG_NULL = 0x0010,
            TPM_ALG_SM3_256 = 0x0012,
            TPM_ALG_RSASSA = 0x0014,
            TPM_ALG_ECDAA = 0x001A,
            TPM_ALG_ECC = 0x0023,
            TPM_ALG_SYMCIPHER = 0x0025,
            TPM_ALG_CTR = 0x0040,
            TPM_ALG_OFB = 0x0041,
            TPM_ALG_CBC = 0x0042,
            TPM_ALG_CFB = 0x0043,
            TPM_ALG_ECB = 0x0044,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::marker::Copy for TpmAlgorithms { }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::core::clone::Clone for TpmAlgorithms {
            #[inline]
            fn clone(&self) -> TpmAlgorithms { { *self } }
        }
        pub enum TpmSE {
            TPM_SE_HMAC = 0x00,
            TPM_SE_POLICY = 0x01,
            TPM_SE_TRIAL = 0x03,
        }
        pub trait UsrTpm: Send + Sync {
            /// Create a clone of the TPM interface that points to the same driver.
            fn clone_usrtpm(&self)
            -> Box<dyn UsrTpm>;
            /// ## Locality related functions
            ///
            /// Locality tells the TPM where the command originated.
            /// Validates the TPM locality, basically means that TPM is ready to listen for commands and
            /// perform operation in this locality.
            /// Ref: https://ebrary.net/24811/computer_science/locality_command
            fn tpm_validate_locality(&self, locality: u32)
            -> bool;
            /// Explicitly giveup locality. This may not be useful if there is only a single process/user using
            /// TPM in an OS. In multi-user scenario, this is more applicable.
            fn relinquish_locality(&self, locality: u32)
            -> bool;
            fn tpm_deactivate_all_localities(&self)
            -> bool;
            /// Requests the TPM to switch to the locality we choose and wait for TPM to acknowledge our
            /// request
            fn tpm_request_locality(&self, locality: u32)
            -> bool;
            /// Reads the TPM ID from device register
            fn read_tpm_id(&self, locality: u32);
            /// Reads the burst_count from TPM register. Burst count is the amount of bytes the TPM device is
            /// capable of handling in oneshot.
            fn tpm_get_burst(&self, locality: u32)
            -> u16;
            /// Busy-wait in a loop for a particular status flag to be set
            fn wait_for_status_flag(&self, locality: u32, flag: u8,
                                    timeout_ms: usize)
            -> bool;
            /// Writes data to the TPM FIFO.
            /// Here, `data.len < burst_count`
            fn tpm_write_data(&self, locality: u32, data: &[u8])
            -> usize;
            /// Checks TPM status register to see if there is any data available
            fn is_data_available(&self, locality: u32)
            -> bool;
            /// Read data from TPM
            /// * Wait for data to be available
            /// * Receive as much as burst_count
            fn tpm_read_data(&self, locality: u32, data: &mut [u8])
            -> usize;
            /// Wrapper for `tpm_read_data`
            /// This function first tries to read TPM_HEADER_SIZE bytes from the TPM to determine the length of
            /// payload data.
            /// Then it issues a second read for the length of payload data subtract TPM_HEADER_SIZE
            /// Payload consists of the argument that was sent to the TPM during tpm_send_data and the response
            fn tpm_recv_data(&self, locality: u32, buf: &mut Vec<u8>,
                             rc: &mut u32)
            -> usize;
            /// Wrapper for `tpm_write_data`
            /// This function waits for TPM to be in a state to accept commands before writing data to FIFO.
            fn tpm_send_data(&self, locality: u32, buf: &mut Vec<u8>)
            -> usize;
            /// Transmit command to a TPM.
            /// This function does a bi-directional communication with TPM.
            /// First, it sends a command with headers
            /// If successful, try to read the response buffer from TPM
            fn tpm_transmit_cmd(&self, locality: u32, buf: &mut Vec<u8>);
            /// Table 3:68 - TPM2_GetRandom Command
            /// Get a random number from TPM.
            /// `num_octets` represents the length of the random number in bytes
            fn tpm_get_random(&self, locality: u32, num_octets: usize)
            -> bool;
            /// Table 3:114 - TPM2_PCR_Read Command
            /// Read a PCR register.
            /// Since the communication channel between the process and the TPM is untrusted,
            /// TPM2_Quote should be the command to retreive PCR values, not TPM2_PCR_Read
            fn tpm_pcr_read(&self, locality: u32, pcr_idx: usize, hash: u16,
                            digest_size: &mut u16, digest: &mut Vec<u8>)
            -> bool;
            /// Obtain information about banks that are allocated in TPM
            fn tpm_init_bank_info(&self, locality: u32, hash_alg: u16)
            -> TpmBankInfo;
            /// Table 3:208 - TPM2_PCR_GetCapability Command
            /// Obtain the banks that are allocated in TPM
            /// TODO: Return true/false, not structure
            fn tpm_get_pcr_allocation(&self, locality: u32)
            -> TpmDevInfo;
            /// Table 3:110 - TPM2_PCR_Read Command
            /// Extend PCR register.
            /// The value sent to the TPM will be concatenated with the original value and hashed.
            fn tpm_pcr_extend(&self, locality: u32, tpm_info: &TpmDevInfo,
                              pcr_idx: usize, digest_values: Vec<TpmTHa>)
            -> bool;
            /// Table 3:78 - TPM2_HashSequenceStart Command
            /// Conduct hash calculation in TPM
            fn tpm_hash_sequence_start(&self, locality: u32,
                                       hash: TpmAlgorithms, object: &mut u32)
            -> bool;
            /// Table 3:80 - TPM2_SequenceUpdate
            /// Update hash calculation in TPM
            fn tpm_sequence_update(&self, locality: u32, object: u32,
                                   buffer: Vec<u8>)
            -> bool;
            /// Table 3:82 - TPM2_SequenceComplete
            /// Finalize hash calculation in TPM
            fn tpm_sequence_complete(&self, locality: u32, object: u32,
                                     buffer: Vec<u8>, hash_size: &mut u16,
                                     hash: &mut Vec<u8>)
            -> bool;
            /// Table 3:62 - TPM2_Hash
            /// Generic hash calculation in TPM when data size is known
            fn tpm_hash(&self, locality: u32, hash: TpmAlgorithms,
                        buffer: Vec<u8>, hash_size: &mut u16,
                        hash_val: &mut Vec<u8>)
            -> bool;
            /// Table 3:164 - TPM2_PCR_CreatePrimary Command
            /// Create Primary Key.
            /// This includes Storate Root Keys and Attestation Identity Keys.
            fn tpm_create_primary(&self, locality: u32,
                                  pcr_idx: Option<usize>, unique_base: &[u8],
                                  restricted: bool, decrypt: bool, sign: bool,
                                  parent_handle: &mut u32,
                                  pubkey_size: &mut usize,
                                  pubkey: &mut Vec<u8>)
            -> bool;
            /// Table 3:15 - TPM2_StartAuthSession Command
            /// Start Authenticated Session and returns a session handle
            fn tpm_start_auth_session(&self, locality: u32,
                                      session_type: TpmSE, nonce: Vec<u8>,
                                      session_handle: &mut u32)
            -> bool;
            /// Table 3:132 - TPM2_PolicyPCR Command
            /// Bind a policy to a particular PCR
            fn tpm_policy_pcr(&self, locality: u32, session_handle: u32,
                              digest: Vec<u8>, pcr_idx: usize)
            -> bool;
            /// Table 3:156 - TPM2_PolicyGetDigest Command
            /// Get Policy digest from current policy
            fn tpm_policy_get_digest(&self, locality: u32,
                                     session_handle: u32,
                                     policy_digest: &mut Vec<u8>)
            -> bool;
            /// Table 3:19 - TPM2_Create Command
            /// Create child key
            fn tpm_create(&self, locality: u32, pcr_idx: Option<usize>,
                          parent_handle: u32, policy: Vec<u8>,
                          sensitive_data: Vec<u8>, restricted: bool,
                          decrypt: bool, sign: bool,
                          out_private: &mut Vec<u8>, out_public: &mut Vec<u8>)
            -> bool;
            /// Table 3:21 - TPM2_Load Command
            /// Load objects into the TPM.
            /// The TPM2B_PUBLIC and TPM2B_PRIVATE objects created by the TPM2_Create command
            /// are to be loaded.
            fn tpm_load(&self, locality: u32, parent_handle: u32,
                        in_private: Vec<u8>, in_public: Vec<u8>,
                        item_handle: &mut u32)
            -> bool;
            /// Table 3:31 - TPM2_Unseal Command
            /// Unseal data sealed via TPM_CC_CREATE
            fn tpm_unseal(&self, locality: u32, session_handle: u32,
                          item_handle: u32, out_data: &mut Vec<u8>)
            -> bool;
            /// Table 3:90 - TPM2_Quote
            /// Generate Quote.
            /// Since the communication channel between the process and the TPM is untrusted,
            /// TPM2_Quote should be the command to retreive PCR values, not TPM2_PCR_Read
            fn tpm_quote(&self, locality: u32, handle: u32, hash: u16,
                         nonce: Vec<u8>, pcr_idxs: Vec<usize>,
                         out_pcr_digest: &mut Vec<u8>, out_sig: &mut Vec<u8>)
            -> bool;
            /// Table 3:198 - TPM2_FlushContext Command
            /// Remove loaded objects, sequence objects, and/or sessions from TPM memory
            fn tpm_flush_context(&self, locality: u32, flush_handle: u32)
            -> bool;
        }
    }
    pub use usr_tpm::*;
    pub use tpm_dev::*;
}
