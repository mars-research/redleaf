use alloc::sync::Arc;
use spin::Mutex;

use libsyscalls::sync::CondVar;
use usr_interface::vfs::{ErrorKind, Result};

use crate::opened_file::{FileType, OpenedFile};

const PIPESIZE: usize = 512;

struct PipeInternal {
    data: [u8; PIPESIZE],
    nread: usize,    // number of bytes read
    nwrite: usize,   // number of bytes written
    readopen: bool,  // read fd is still open
    writeopen: bool, // write fd is still open
}

impl core::fmt::Debug for PipeInternal {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.debug_struct("Pipe")
            .field("nread", &self.nread)
            .field("nwrite", &self.nwrite)
            .field("readopen", &self.readopen)
            .field("writeopen", &self.writeopen)
            .finish()
    }
}

impl PipeInternal {
    fn new() -> Self {
        Self {
            data: [0u8; PIPESIZE],
            nread: 0,
            nwrite: 0,
            readopen: true,
            writeopen: true,
        }
    }
}

pub struct Pipe {
    pipe: Mutex<PipeInternal>,
    can_read: CondVar,
    can_write: CondVar,
}

impl core::fmt::Debug for Pipe {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(fmt, "{:?}", self.pipe)
    }
}

impl Pipe {
    fn new() -> Self {
        Self {
            pipe: Mutex::new(PipeInternal::new()),
            can_read: CondVar::new(),
            can_write: CondVar::new(),
        }
    }

    pub fn pipealloc() -> (Arc<OpenedFile>, Arc<OpenedFile>) {
        let pipe = Arc::new(Pipe::new());
        let f0 = OpenedFile::new(
            FileType::Pipe { pipe: pipe.clone() },
            /*readable=*/ true,
            /*writable=*/ false,
        );
        let f1 = OpenedFile::new(
            FileType::Pipe { pipe },
            /*readable=*/ false,
            /*writable=*/ true,
        );
        (Arc::new(f0), Arc::new(f1))
    }

    pub fn close(&self, writable: bool) {
        let mut pipe = self.pipe.lock();
        if writable {
            pipe.writeopen = false;
            self.can_read.wakeup();
        } else {
            pipe.readopen = false;
            self.can_write.wakeup();
        }

        // The arc will drop the pipe if the ref count is zero
    }

    // TODO: handle myproc()->killed
    pub fn write(&self, data: &[u8]) -> Result<usize> {
        let pred = |pipe: &mut PipeInternal| -> bool {
            // Stop waiting if pipe is closed for reading or the buffer is not full
            if !pipe.readopen || pipe.nwrite < pipe.nread + PIPESIZE {
                return true;
            }

            // Notify the read end to read
            self.can_read.wakeup();
            false
        };

        let mut pipe = self.can_write.sleep_until(&self.pipe, pred);
        if !pipe.readopen {
            return Err(ErrorKind::BrokenPipe);
        }
        for c in data.iter() {
            // Wait while the buffer is full
            if pipe.nwrite == pipe.nread + PIPESIZE {
                drop(pipe);
                pipe = self.can_write.sleep_until(&self.pipe, pred);
            }

            // If the readend is close, no one will wake us up
            if !pipe.readopen && pipe.nwrite == pipe.nread + PIPESIZE {
                return Err(ErrorKind::BrokenPipe);
            }

            // Copy the data to the buffer
            let nwrite = pipe.nwrite; // Copy it here so the borrow checker won't complain
            pipe.data[nwrite % PIPESIZE] = *c;
            pipe.nwrite += 1;
        }

        // Release the lock
        drop(pipe);

        // Wake up the read end
        self.can_read.wakeup();

        Ok(data.len())
    }

    // TODO: handle myproc()->killed
    pub fn read(&self, data: &mut [u8]) -> Result<usize> {
        let pred = |pipe: &mut PipeInternal| {
            // Stop waiting if pipe is closed for writing or the buffer is not empty
            return !pipe.writeopen || pipe.nread != pipe.nwrite;
        };

        // Sleep until there's something to read or write end is closed
        let mut pipe = self.can_read.sleep_until(&self.pipe, pred);

        // Copy data over
        let mut bytes_read = 0;
        for c in data.iter_mut() {
            if pipe.nread == pipe.nwrite {
                break;
            }

            *c = pipe.data[pipe.nread % PIPESIZE];
            pipe.nread += 1;
            bytes_read += 1;
        }

        // Release the lock
        drop(pipe);

        // Wakeup the write end
        self.can_write.wakeup();

        Ok(bytes_read)
    }
}
