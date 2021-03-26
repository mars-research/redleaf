use crate::params;
use byteorder::ByteOrder;
use std::mem::size_of;

/*
   I am using a struct to mimic enum behavior
   since I have two tags, Indirect and Block,
   which have the same value.

   Should I stick with this, or use a proper enum
   and give a generic name for Indirect and Block
*/

#[non_exhaustive]
pub struct LayerType;

impl LayerType {
    pub const Indirect: usize = params::BSIZE;
    pub const Direct: usize = (params::NDIRECT * size_of::<u32>()) + size_of::<u32>();
    pub const Block: usize = params::BSIZE;
}

pub struct Layer {
    buffer: Vec<u8>,
    layer_type: usize,
}

impl Layer {
    pub fn new(size: usize) -> Self {
        Layer {
            layer_type: size,
            buffer: vec![0; size],
        }
    }

    pub fn set(&mut self, data: u32, idx: usize) {
        let index = idx * 4;
        byteorder::LittleEndian::write_u32(&mut self.buffer[index..index + 4], data);
    }

    pub fn get(&self, idx: usize) -> u32 {
        let index = idx * 4;
        byteorder::LittleEndian::read_u32(&self.buffer[index..index + 4])
    }

    pub fn is_block_empty(&self, idx: usize) -> bool {
        self.get(idx) == 0
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buffer.as_mut_slice()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }
}
