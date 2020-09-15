// Adapted from the Redox OS Project:
//
//   Copyright (c) 2017 Redox OS Developers
//   
//   MIT License
//   
//   Permission is hereby granted, free of charge, to any person obtaining
//   a copy of this software and associated documentation files (the
//   "Software"), to deal in the Software without restriction, including
//   without limitation the rights to use, copy, modify, merge, publish,
//   distribute, sublicense, and/or sell copies of the Software, and to
//   permit persons to whom the Software is furnished to do so, subject to
//   the following conditions:
//   
//   The above copyright notice and this permission notice shall be
//   included in all copies or substantial portions of the Software.
//   
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
//   EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//   MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
//   NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
//   LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
//   WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use super::{Dma, Mmio, DmaAllocator};
use super::zeroed_allocator;
use libsyscalls::errors::Result;

#[repr(packed)]
#[derive(Debug)]
pub struct HbaPrdtEntry {
    pub dba: Mmio<u64>, // Data base address
    _rsv0: Mmio<u32>, // Reserved
    pub dbc: Mmio<u32>, // Byte count, 4M max, interrupt = 1
}

#[repr(packed)]
pub struct HbaCmdTable {
    // 0x00
    pub cfis: [Mmio<u8>; 64], // Command FIS

    // 0x40
    pub acmd: [Mmio<u8>; 16], // ATAPI command, 12 or 16 bytes

    // 0x50
    _rsv: [Mmio<u8>; 48], // Reserved

    // 0x80
    pub prdt_entry: [HbaPrdtEntry; 65536], // Physical region descriptor table entries, 0 ~ 65535
}

#[repr(packed)]
#[derive(Debug)]
pub struct HbaCmdHeader {
    // DW0
    pub cfl: Mmio<u8>, /* Command FIS length in DWORDS, 2 ~ 16, atapi: 4, write - host to device: 2, prefetchable: 1 */
    _pm: Mmio<u8>, // Reset - 0x80, bist: 0x40, clear busy on ok: 0x20, port multiplier

    pub prdtl: Mmio<u16>, // Physical region descriptor table length in entries

    // DW1
    _prdbc: Mmio<u32>, // Physical region descriptor byte count transferred

    // DW2, 3
    pub ctba: Mmio<u64>, // Command table descriptor base address

    // DW4 - 7
    _rsv1: [Mmio<u32>; 4], // Reserved
}

zeroed_allocator!([HbaCmdHeader; 32]); // clb
zeroed_allocator!(HbaCmdTable); // ctba
zeroed_allocator!([u8; 256]); // fb
zeroed_allocator!([u8; 256 * 512]); // buf
zeroed_allocator!([u8; 512 * 512]); // buf
zeroed_allocator!([u16; 256]); // identify

pub fn allocate_dma<T>() -> Result<Dma<T>>
    where T: DmaAllocator
{
    T::allocate()
}
