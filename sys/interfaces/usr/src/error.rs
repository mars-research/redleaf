// Modified from https://github.com/rust-lang/rust/blob/master/src/libstd/io/error.rs

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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
}

// impl ErrorKind {
//     pub(crate) fn as_str(&self) -> &'static str {
//         match *self {
//             ErrorKind::FileNotFound => "file not found",
//             ErrorKind::PermissionDenied => "permission denied",
//             ErrorKind::ConnectionRefused => "connection refused",
//             ErrorKind::ConnectionReset => "connection reset",
//             ErrorKind::ConnectionAborted => "connection aborted",
//             ErrorKind::NotConnected => "not connected",
//             ErrorKind::AddrInUse => "address in use",
//             ErrorKind::AddrNotAvailable => "address not available",
//             ErrorKind::BrokenPipe => "broken pipe",
//             ErrorKind::FileAlreadyExists => "file already exists",
//             ErrorKind::WouldBlock => "operation would block",
//             ErrorKind::InvalidInput => "invalid input parameter",
//             ErrorKind::InvalidData => "invalid data",
//             ErrorKind::TimedOut => "timed out",
//             ErrorKind::WriteZero => "write zero",
//             ErrorKind::Interrupted => "operation interrupted",
//             ErrorKind::Other => "other os error",
//             ErrorKind::UnexpectedEof => "unexpected end of file",
//             ErrorKind::FormatError => "format error when doing write_fmt",
//             ErrorKind::TooManyOpenedFiles => "too many opened files",
//             ErrorKind::InvalidCTTSId => "invalid cross-thread-temp-storage id",
//             ErrorKind::InvalidFileDescriptor => "invalid file descriptor",
//             ErrorKind::InvalidMajor => "invalid device major number",
//             ErrorKind::ICacheExhausted => "inode cache ran out of nodes",
//             ErrorKind::OutOfINode => "no more free inode that we can allocate",
//             ErrorKind::InvalidFileType => "invalid file type",
//             ErrorKind::DirectoryExhausted => "directory exhausted",
//         }
//     }
// }

impl core::convert::From<RpcError> for ErrorKind {
    fn from(_: RpcError) -> Self {
        Self::RpcError
    }
}