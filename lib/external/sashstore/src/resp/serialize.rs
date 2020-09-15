//! RESP serialize
#![allow(unused)]

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use log::error;

use super::value::Value;
use super::DecodeError;

/// up to 512 MB in length
const RESP_MAX_SIZE: i64 = 512 * 1024 * 1024;
const CRLF_BYTES: &'static [u8] = b"\r\n";
const NULL_BYTES: &'static [u8] = b"$-1\r\n";
const NULL_ARRAY_BYTES: &'static [u8] = b"*-1\r\n";

/// Encodes RESP value to RESP binary buffer.
///
/// Avoids allocating the buffer by passing an existing one in.
pub fn encode_with_buf(mut res: Vec<u8>, value: &Value) -> Vec<u8> {
    buf_encode(value, &mut res);
    res
}

/// Encodes RESP value to RESP binary buffer.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::new();
    buf_encode(value, &mut res);
    res
}

/// Encodes a slice of string to RESP binary buffer.
/// It is use to create a request command on redis client.
pub fn encode_slice(slice: &[&str]) -> Vec<u8> {
    let array: Vec<Value> = slice
        .iter()
        .map(|string| Value::Bulk(string.to_string()))
        .collect();
    let mut res: Vec<u8> = Vec::new();
    buf_encode(&Value::Array(array), &mut res);
    res
}

#[inline]
fn buf_encode(value: &Value, buf: &mut Vec<u8>) {
    match *value {
        Value::Null => {
            buf.extend_from_slice(NULL_BYTES);
        }
        Value::NullArray => {
            buf.extend_from_slice(NULL_ARRAY_BYTES);
        }
        Value::String(ref val) => {
            buf.push(b'+');
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Error(ref val) => {
            buf.push(b'-');
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::StaticError(ref val) => {
            buf.push(b'-');
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Integer(ref val) => {
            buf.push(b':');
            buf.extend_from_slice(val.to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Bulk(ref val) => {
            buf.push(b'$');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            buf.extend_from_slice(val.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::BufBulk(ref val) => {
            buf.push(b'$');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            buf.extend_from_slice(val);
            buf.extend_from_slice(CRLF_BYTES);
        }
        Value::Array(ref val) => {
            buf.push(b'*');
            buf.extend_from_slice(val.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            for item in val {
                buf_encode(item, buf);
            }
        }
    }
}

/// A streaming RESP Decoder.
#[derive(Debug)]
pub struct Decoder {
    buf_bulk: bool,
    reader: VecDeque<u8>,
}

impl Into<Vec<u8>> for Decoder {
    fn into(self) -> Vec<u8> {
        self.reader.into()
    }
}

impl Decoder {
    /// Creates a Decoder instance with given BufReader for decoding the RESP buffers.
    pub fn new(reader: VecDeque<u8>) -> Self {
        Decoder {
            buf_bulk: false,
            reader: reader,
        }
    }

    /// Creates a Decoder instance with given BufReader for decoding the RESP buffers.
    /// The instance will decode bulk value to buffer bulk.
    pub fn with_buf_bulk(reader: VecDeque<u8>) -> Self {
        Decoder {
            buf_bulk: true,
            reader: reader,
        }
    }

    // Conversion of: self.reader.read_until(b'\n', &mut res)?;
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> usize {
        let mut popped = 0;
        loop {
            match self.reader.pop_front() {
                None => return popped,
                Some(c) => {
                    popped += 1;
                    buf.push(c);

                    if c == byte {
                        return popped;
                    }
                }
            }
        }
    }

    // Conversion of self.reader.read_exact(buf.as_mut_slice())?;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), DecodeError> {
        for i in 0..buf.len() {
            match self.reader.pop_front() {
                None => return Err(DecodeError::UnexpectedEof),
                Some(c) => buf[i] = c,
            }
        }
        Ok(())
    }

    /// It will read buffers from the inner BufReader, decode it to a Value.
    pub fn decode(&mut self) -> Result<Value, DecodeError> {
        let mut res: Vec<u8> = Vec::with_capacity(16);
        self.read_until(b'\n', &mut res);
        let len = res.len();
        if len < 3 {
            error!("len < 3");
            return Err(DecodeError::InvalidInput);
        }
        if !is_crlf(res[len - 2], res[len - 1]) {
            error!("!is_crlf(res[len - 2], res[len - 1])");
            return Err(DecodeError::InvalidInput);
        }

        let bytes = res[1..len - 2].as_ref();
        match res[0] {
            // Value::String
            b'+' => parse_string(bytes).map(Value::String),
            // Value::Error
            b'-' => parse_string(bytes).map(Value::Error),
            // Value::Integer
            b':' => parse_integer(bytes).map(Value::Integer),
            // Value::Bulk
            b'$' => {
                let int = parse_integer(bytes)?;
                if int == -1 {
                    // Null bulk
                    return Ok(Value::Null);
                }
                if int < -1 || int >= RESP_MAX_SIZE {
                    error!("int < -1 || int >= RESP_MAX_SIZE");
                    return Err(DecodeError::InvalidInput);
                }

                let mut buf: Vec<u8> = Vec::new();
                let int = int as usize;
                buf.resize(int + 2, 0);
                self.read_exact(buf.as_mut_slice())?;
                if !is_crlf(buf[int], buf[int + 1]) {
                    error!("!is_crlf(buf[int], buf[int + 1])");
                    return Err(DecodeError::InvalidInput);
                }
                buf.truncate(int);
                if self.buf_bulk {
                    return Ok(Value::BufBulk(buf));
                }
                parse_string(buf.as_slice()).map(Value::Bulk)
            }
            // Value::Array
            b'*' => {
                let int = parse_integer(bytes)?;
                if int == -1 {
                    // Null array
                    return Ok(Value::NullArray);
                }
                if int < -1 || int >= RESP_MAX_SIZE {
                    return Err(DecodeError::InvalidInput);
                }

                let mut array: Vec<Value> = Vec::with_capacity(int as usize);
                for _ in 0..int {
                    let val = self.decode()?;
                    array.push(val);
                }
                Ok(Value::Array(array))
            }
            _prefix => Err(DecodeError::InvalidType),
        }
    }
}

#[inline]
fn is_crlf(a: u8, b: u8) -> bool {
    a == b'\r' && b == b'\n'
}

#[inline]
fn parse_string(bytes: &[u8]) -> Result<String, DecodeError> {
    String::from_utf8(bytes.to_vec()).map_err(|_err| DecodeError::InvalidData)
}

#[inline]
fn parse_integer(bytes: &[u8]) -> Result<i64, DecodeError> {
    let str_integer = parse_string(bytes)?;
    (str_integer.parse::<i64>()).map_err(|_err| DecodeError::InvalidData)
}
