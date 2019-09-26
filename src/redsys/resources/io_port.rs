// The intention is to write the names in full for clarity
use x86;

// An IOPort grants outb/inb access to an X86 I/O Port
#[derive(Debug)]
pub struct IOPort {
    // Private fields
    _port: u16,
    _can_read: bool,
    _can_write: bool,
}

impl IOPort {
    pub unsafe fn new(port: u16, can_read: bool, can_write: bool) -> IOPort {
        // FIXME: Have a standarized data structure to declare capabilities
        IOPort {
            _port: port,
            _can_read: can_read,
            _can_write: can_write,
        }
    }

    /// Whether the port can be read
    pub fn can_read(&self) -> bool {
        self._can_read
    }

    /// Whether the port can be written to
    pub fn can_write(&self) -> bool {
        self._can_write
    }

    /// Write 8 bits to port
    pub fn outb(&self, val: u8) -> Result<u8, &'static str> {
        if !self._can_write {
            Err("Policy forbids writing to port")
        } else {
            unsafe {
                x86::io::outb(self._port, val);
            }
            Ok(val)
        }
    }

    /// Read 8 bits from port
    pub fn inb(&self) -> Result<u8, &'static str> {
        if !self._can_read {
            Err("Policy forbids reading from port")
        } else {
            let val = unsafe { x86::io::inb(self._port) };
            Ok(val)
        }
    }

    // FIXME: The following methods should be implemented in the x86 crate

    /// Write dword string to port
    pub fn outsl(&self, val: &[u32]) -> Result<(), &'static str> {
        if !self._can_write {
            Err("Policy forbids writing to port")
        } else {
            unsafe {
                asm!("cld; rep; outsd"
                     : // output
                     : "{dx}"(self._port), "{esi}"(val as *const _ as *const () as usize), "{ecx}"(val.len() as u32) // input
                     : // clobber
                     : "intel" // options
                );
            }
            Ok(())
        }
    }

    /// Read dword string to port
    pub fn insl(&self, val: &mut [u32]) -> Result<(), &'static str> {
        if !self._can_read {
            Err("Policy forbids reading from port")
        } else {
            unsafe {
                asm!("cld; rep; outsd"
                     : "={esi}"(val as *const _ as *const () as usize) // output
                     : "{dx}"(self._port), "{ecx}"(val.len() as u32) // input
                     : // clobber
                     : "intel" // options
                );
            }
            Ok(())
        }
    }
}
