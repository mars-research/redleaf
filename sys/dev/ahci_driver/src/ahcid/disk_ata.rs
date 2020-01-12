// Intel SATA AHCI v1.3.1
// Section 9.3.1.1.5.1
// Non-queued commands may use any command slot. The HBA guarantees that command issue order is
// preserved, so software does not need to ensure any ordering of command slots.
// To issue a non-queued command, the host should:
// 1. Select an unused command slot
// 2. Build the command table and command header
// 3. Set PxFBS.DEV to the value of the Port Multiplier port field in the command header
// 4. Set the bit in PxCI that corresponds to the command slot being used

// One Port could have several devices attached to it if there's a multiplexer.
// For now we can assume that there's only one device on a port.

// Each port has a command list containing serveral(32) command header slots.
//  This command list can be used by system software and the HBA even when non-queued commands
//  need to be transferred. System software can still place multiple commands in the list, whether DMA, PIO,
//  or ATAPI, and the HBA will walk the list transferring them.

// Each command header points to a variable sized(up to 65,535) Physical Region Descriptor Table(PRDT)

// Each entry(item) in PRDT contains a pointer(physical address) to the buffer(up to 4MB) that the device can DMA to

use core::ptr;

use libsyscalls::errors::Result;
use libsyscalls::syscalls::sys_yield;

use libdma::Dma;
use libdma::ahci::{HbaCmdTable, HbaCmdHeader};
use libdma::ahci::allocate_dma;
use super::hba::HbaPort;
use super::Disk;

// Maximun number of sectors per PRDT entry
pub const MAX_SECTORS_PER_PRDT_ENTRY: usize = 8192;
// The size of a sector(some call it block) of the disk in bytes
pub const SECTOR_SIZE: usize = 512;
// Maximun number of bytes per PRDT entry
pub const MAX_BYTES_PER_PRDT_ENTRY: usize = MAX_SECTORS_PER_PRDT_ENTRY * SECTOR_SIZE;
// Maximun number of PRDT entries in a PRDTable
pub const MAX_PRDT_ENTRIES: usize = 65_535;

enum BufferKind<'a> {
    Read(&'a mut [u8]),
    Write(&'a [u8]),
}

struct Request {
    address: usize,
    total_sectors: usize,
    sector: usize,
    running_opt: Option<(u32, usize)>,
}

pub struct DiskATA {
    id: usize,
    port: HbaPort,
    size: u64,
    request_opt: Option<Request>,
    clb: Dma<[HbaCmdHeader; 32]>,
    ctbas: [Dma<HbaCmdTable>; 32],
    _fb: Dma<[u8; 256]>,
}

impl DiskATA {
    pub fn new(id: usize, mut port: HbaPort) -> Result<Self> {
        let mut clb = allocate_dma()?;
        let mut ctbas = [
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
            allocate_dma()?, allocate_dma()?, allocate_dma()?, allocate_dma()?,
        ];
        let mut fb = allocate_dma()?;

        port.init(&mut clb, &mut ctbas, &mut fb);

        let size = port.identify(&mut clb, &mut ctbas).unwrap_or(0);

        Ok(DiskATA {
            id: id,
            port: port,
            size: size,
            request_opt: None,
            clb: clb,
            ctbas: ctbas,
            _fb: fb,
        })
    }

    fn request(&mut self, block: u64, mut buffer_kind: BufferKind) -> Result<Option<usize>> {
        let (write, address, total_sectors) = match buffer_kind {
            BufferKind::Read(ref buffer) => {
                assert!(buffer.len()%512 == 0, "Must read a multiple of block size number of bytes");
                (false, buffer.as_ptr() as usize, buffer.len()/512)
            },
            BufferKind::Write(ref buffer) => {
                assert!(buffer.len()%512 == 0, "Must read a multiple of block size number of bytes");
                (true, buffer.as_ptr() as usize, buffer.len()/512)
            },
        };
        assert!(total_sectors <= MAX_SECTORS_PER_PRDT_ENTRY);

        //TODO: Go back to interrupt magic
        let use_interrupts = false;
        loop {
            let mut request = match self.request_opt.take() {
                Some(request) => if address == request.address && total_sectors == request.total_sectors {
                    // Keep servicing current request
                    request
                } else {
                    // Have to wait for another request to finish
                    self.request_opt = Some(request);
                    return Ok(None);
                },
                None => {
                    // Create new request
                    Request {
                        address,
                        total_sectors,
                        sector: 0,
                        running_opt: None,
                    }
                }
            };

            // Finish a previously running request
            if let Some(running) = request.running_opt.take() {
                if self.port.ata_running(running.0) {
                    // Continue waiting for request
                    request.running_opt = Some(running);
                    self.request_opt = Some(request);
                    if use_interrupts {
                        return Ok(None);
                    } else {
                        sys_yield();
                        continue;
                    }
                }

                self.port.ata_stop(running.0)?;

                request.sector += running.1;
            }

            if request.sector < request.total_sectors {
                // Start a new request
                let sectors = if request.total_sectors - request.sector >= 255 {
                    255
                } else {
                    request.total_sectors - request.sector
                };

                if let Some(slot) = self.port.ata_dma(block + request.sector as u64, sectors, write, &mut self.clb, &mut self.ctbas, address) {
                    request.running_opt = Some((slot, sectors));
                }

                self.request_opt = Some(request);

                if use_interrupts {
                    return Ok(None);
                } else {
                    sys_yield();
                    continue;
                }
            } else {
                // Done
                return Ok(Some(request.sector * 512));
            }
        }
    }
}

impl Disk for DiskATA {
    fn id(&self) -> usize {
        self.id
    }

    fn size(&mut self) -> u64 {
        self.size
    }

    fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<Option<usize>> {
        self.request(block, BufferKind::Read(buffer))
    }

    fn write(&mut self, block: u64, buffer: &[u8]) -> Result<Option<usize>> {
        self.request(block, BufferKind::Write(buffer))
    }

    fn block_length(&mut self) -> Result<u32> {
        Ok(512)
    }
}
