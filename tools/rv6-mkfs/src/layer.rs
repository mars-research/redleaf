use crate::params;
use byteorder::ByteOrder;
use std::mem::size_of;

#[repr(usize)]
#[derive(Clone, Copy)]
pub enum LayerType {
    Direct = params::NDIRECT,
    Indirect = params::BSIZE,
}

pub struct Layer {
    buffer: Vec<u8>,
    layer_type: LayerType,
}

impl Layer {
    pub fn new(size: LayerType) -> Self {
        Layer {
            layer_type: size.clone(),
            buffer: vec![0; size.clone() as usize],
        }
    }

    pub fn update(&mut self, data: u32, idx: usize) {
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
