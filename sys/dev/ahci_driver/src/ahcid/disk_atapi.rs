#![allow(dead_code)]

use core::ptr;

use byteorder::{ByteOrder, BigEndian};

use libsyscalls::errors::{Result, EBADF, Error};

use libdma::Dma;
use super::hba::HbaPort;
use libdma::ahci::{HbaCmdTable, HbaCmdHeader};
use libdma::ahci::allocate_dma;
use super::Disk;

const SCSI_READ_CAPACITY: u8 = 0x25;
const SCSI_READ10: u8 = 0x28;

pub struct DiskATAPI {
    id: usize,
    port: &'static mut HbaPort,
    size: u64,
    clb: Dma<[HbaCmdHeader; 32]>,
    ctbas: [Dma<HbaCmdTable>; 32],
    _fb: Dma<[u8; 256]>,
    // Just using the same buffer size as DiskATA
    // Although the sector size is different (and varies)
    buf: Dma<[u8; 256 * 512]>
}

impl DiskATAPI {
    pub fn new(id: usize, port: &'static mut HbaPort) -> Result<Self> {
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
        let buf = allocate_dma()?;

        port.init(&mut clb, &mut ctbas, &mut fb);

        let size = unsafe { port.identify_packet(&mut clb, &mut ctbas).unwrap_or(0) };

        Ok(DiskATAPI {
            id: id,
            port: port,
            size: size,
            clb: clb,
            ctbas: ctbas,
            _fb: fb,
            buf: buf
        })
    }

    fn read_capacity(&mut self) -> Result<(u32, u32)> {
        // TODO: only query when needed (disk changed)

        let mut cmd = [0; 16];
        cmd[0] = SCSI_READ_CAPACITY;
        self.port.atapi_dma(&cmd, 8, &mut self.clb, &mut self.ctbas, &mut self.buf)?;

        // Instead of a count, contains number of last LBA, so add 1
        let blk_count = BigEndian::read_u32(&self.buf[0..4]) + 1;
        let blk_size = BigEndian::read_u32(&self.buf[4..8]);

        Ok((blk_count, blk_size))
    }
}

impl Disk for DiskATAPI {
    fn id(&self) -> usize {
        self.id
    }

    fn size(&mut self) -> u64 {
        match self.read_capacity() {
            Ok((blk_count, blk_size)) => (blk_count as u64) * (blk_size as u64),
            Err(_) => 0 // XXX
        }
    }

    fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<Option<usize>> {
        // TODO: Handle audio CDs, which use special READ CD command

        let blk_len = self.block_length()?;
        let sectors = buffer.len() as u32 / blk_len;

        fn read10_cmd(block: u32, count: u16) -> [u8; 16] {
            let mut cmd = [0; 16];
            cmd[0] = SCSI_READ10;
            BigEndian::write_u32(&mut cmd[2..6], block as u32);
            BigEndian::write_u16(&mut cmd[7..9], count as u16);
            cmd
        }

        let mut sector = 0;
        let buf_len = (256 * 512) / blk_len;
        let buf_size = buf_len * blk_len;
        while sectors - sector >= buf_len {
            let cmd = read10_cmd(block as u32 + sector, buf_len as u16);
            self.port.atapi_dma(&cmd, buf_size, &mut self.clb, &mut self.ctbas, &mut self.buf)?;

            unsafe { ptr::copy(self.buf.as_ptr(), buffer.as_mut_ptr().offset(sector as isize * blk_len as isize), buf_size as usize); }

            sector += blk_len;
        }
        if sector < sectors {
            let cmd = read10_cmd(block as u32 + sector, (sectors - sector) as u16);
            self.port.atapi_dma(&cmd, buf_size, &mut self.clb, &mut self.ctbas, &mut self.buf)?;

            unsafe { ptr::copy(self.buf.as_ptr(), buffer.as_mut_ptr().offset(sector as isize * blk_len as isize), ((sectors - sector) * blk_len) as usize); }

            sector += sectors - sector;
        }

        Ok(Some((sector * blk_len) as usize))
    }

    fn write(&mut self, _block: u64, _buffer: &[u8]) -> Result<Option<usize>> {
        Err(Error::new(EBADF)) // TODO: Implement writing
    }

    fn block_length(&mut self) -> Result<u32> {
        Ok(self.read_capacity()?.1)
    }
}
