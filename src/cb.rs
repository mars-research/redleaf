/**
 * Circular buffer implementation
 * 
 * backed by a Vec<T>
 */
use alloc::vec::Vec;

pub struct CircularBuffer<T>
    where T: Copy + Default
{
    buf: Vec<T>,
    head: usize,
    tail: usize,
    size: usize
}

const INITIAL_CAPACITY : usize = 8;

#[derive(Debug)]
pub enum CbError {
    QueueIsFull,
    QueueIsEmpty,
    InvalidAccess,
}

pub type CbRet<T> = Result<T, CbError>;

impl<T> CircularBuffer<T>
    where T: Copy + Default
{

    fn is_queue_full(&self) -> bool {
        (self.tail + 1) & (self.size - 1) == self.head
    }
    
    fn is_queue_empty(&self) -> bool {
        self.tail == self.head
    }
}

impl<T> CircularBuffer<T>
    where T: Copy + Default
{
    
    pub fn new() -> CircularBuffer<T> {
        CircularBuffer::new_with_size(INITIAL_CAPACITY)      
    }
    
    pub fn new_with_size(size: usize) -> CircularBuffer<T> {
        CircularBuffer {
            head: 0,
            tail: 0,
            buf: vec![T::default(); size],
            size: size
        }
    }
    
    pub fn push(&mut self, val: T) -> CbRet<()> {
        if self.is_queue_full() {
            return Err(CbError::QueueIsFull)
        }

        if let Some(elem) = self.buf.get_mut(self.tail) {
            *elem = val;
            self.tail = (self.tail + 1) & (self.size - 1);
        }
        Ok(())
    }
    
    pub fn pop(&mut self) -> CbRet<T> {
        if self.is_queue_empty() {
            return Err(CbError::QueueIsEmpty);
        }
        
        if let Some(val) = self.buf.get(self.head) {
            self.head = (self.head + 1) & (self.size - 1);
            return Ok(*val);
        } else {
            return Err(CbError::InvalidAccess);
        }
    }
}