use byteorder::ByteOrder;
use crate::params;
use std::mem::size_of;

pub enum LayerType {
    // store size in enum, but ctor says cannot borrow?
    Indirect, 
    Direct,
    Block,
}

pub struct Layer {
    pub buffer: Vec<u8>,
    layer_type: LayerType,
}

impl Layer {
    pub fn new(t: LayerType) -> Self {
        match t {
            LayerType::Indirect => {
                return Layer {
                    // normally, this would be a u32 of szie
                    // params::NINDIRECT. But we are representing
                    // as u8, so the size would be params::NINDIRECT * sizeof(u32)
                    // which is equal to params::BSIZE 
                    buffer: vec![0; params::BSIZE],
                    layer_type: t,  
                };
            }
            LayerType::Direct => {
                let u32_size = size_of::<u32>();
                return Layer {
                    buffer: vec![0; u32_size + (params::NDIRECT * u32_size)],
                    layer_type: t,  
                };
            }
            LayerType::Block => {
                return Layer {
                    buffer: vec![0; params::BSIZE],
                    layer_type: t,  
                };
            }
        }
    }

    pub fn set(&mut self, data: u32, idx: usize) {
        let index = idx * 4;
        byteorder::LittleEndian::write_u32(&mut self.buffer[index..index+4], data);
    }

    pub fn get(&self, idx: usize) -> u32 {
        let index = idx * 4;
        byteorder::LittleEndian::read_u32(&self.buffer[index..index+4])
    }

    pub fn is_block_empty(&self, idx:usize) -> bool {
        self.get(idx) == 0
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buffer.as_mut_slice()
    }


    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }
}
