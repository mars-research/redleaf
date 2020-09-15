//! RESP Value
#![allow(unused)]
use super::serialize::encode;
use alloc::string::String;
use alloc::vec::Vec;

use super::DecodeError;

/// Represents a RESP value, see [Redis Protocol specification](http://redis.io/topics/protocol).
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value {
    /// Null bulk reply, `$-1\r\n`
    Null,
    /// Null array reply, `*-1\r\n`
    NullArray,
    /// For Simple Strings the first byte of the reply is "+".
    String(String),
    /// For Errors the first byte of the reply is "-".
    Error(String),
    /// For Errors the first byte of the reply is "-".
    StaticError(&'static str),
    /// For Integers the first byte of the reply is ":".
    Integer(i64),
    /// For Bulk Strings the first byte of the reply is "$".
    Bulk(String),
    /// For Bulk <binary> Strings the first byte of the reply is "$".
    BufBulk(Vec<u8>),
    /// For Arrays the first byte of the reply is "*".
    Array(Vec<Value>),
}

impl Value {
    /// Returns `true` if the value is a `Null` or `NullArray`. Returns `false` otherwise.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// assert_eq!(Value::Null.is_null(), true);
    /// assert_eq!(Value::NullArray.is_null(), true);
    /// assert_eq!(Value::Integer(123).is_null(), false);
    /// ```
    pub fn is_null(&self) -> bool {
        match *self {
            Value::Null | Value::NullArray => true,
            _ => false,
        }
    }

    /// Returns `true` if the value is a `Error`. Returns `false` otherwise.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// assert_eq!(Value::Null.is_error(), false);
    /// assert_eq!(Value::Error("".to_string()).is_error(), true);
    /// ```
    pub fn is_error(&self) -> bool {
        match *self {
            Value::Error(_) => true,
            _ => false,
        }
    }

    /// Encode the value to RESP binary buffer.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// let val = Value::String("OK正".to_string());
    /// assert_eq!(val.encode(), vec![43, 79, 75, 230, 173, 163, 13, 10]);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        encode(self)
    }

    /// Encode the value to RESP string.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// let val = Value::String("OK正".to_string());
    /// assert_eq!(val.to_encoded_string().unwrap(), "+OK正\r\n");
    /// ```
    pub fn to_encoded_string(&self) -> Result<String, DecodeError> {
        let bytes = self.encode();
        String::from_utf8(bytes).map_err(|_err| DecodeError::InvalidData)
    }

    /// Encode the value to beautify formated string.
    /// # Examples
    /// ```
    /// # use self::resp::{Value};
    /// assert_eq!(Value::Null.to_string_pretty(), "(Null)");
    /// assert_eq!(Value::NullArray.to_string_pretty(), "(Null Array)");
    /// assert_eq!(Value::String("OK".to_string()).to_string_pretty(), "OK");
    /// assert_eq!(Value::Error("Err".to_string()).to_string_pretty(), "(Error) Err");
    /// assert_eq!(Value::Integer(123).to_string_pretty(), "(Integer) 123");
    /// assert_eq!(Value::Bulk("Bulk String".to_string()).to_string_pretty(), "\"Bulk String\"");
    /// assert_eq!(Value::BufBulk(vec![]).to_string_pretty(), "(Empty Buffer)");
    /// assert_eq!(Value::BufBulk(vec![0, 100]).to_string_pretty(), "(Buffer) 00 64");
    /// assert_eq!(Value::Array(vec![]).to_string_pretty(), "(Empty Array)");
    /// ```
    ///
    /// A full formated example:
    ///
    /// ```bash
    ///  1) (Null)
    ///  2) (Null Array)
    ///  3) OK
    ///  4) (Error) Err
    ///  5) (Integer) 123
    ///  6) \"Bulk String\"
    ///  7) (Empty Array)
    ///  8) (Buffer) 00 64
    ///  9) 1) (Empty Array)
    ///     2) (Integer) 123
    ///     3) \"Bulk String\"
    /// 10) 1) (Null)
    ///     2) (Null Array)
    ///     3) OK
    ///     4) (Error) Err
    ///     5) (Integer) 123
    ///     6) \"Bulk String\"
    ///     7) (Empty Array)
    ///     8) (Buffer) 00 64
    ///     9) 1) (Empty Array)
    ///        2) (Integer) 123
    ///        3) \"Bulk String\"
    /// 11) (Null)
    /// 12) 1) (Null)
    ///     2) (Null Array)
    ///     3) OK
    ///     4) (Error) Err
    ///     5) (Integer) 123
    ///     6) \"Bulk String\"
    ///     7) (Empty Array)
    ///     8) (Buffer) 00 64
    ///     9) 1) (Empty Array)
    ///        2) (Integer) 123
    ///        3) \"Bulk String\"
    ///    10) 1) (Null)
    ///        2) (Null Array)
    ///        3) OK
    ///        4) (Error) Err
    ///        5) (Integer) 123
    ///        6) \"Bulk String\"
    ///        7) (Empty Array)
    ///        8) (Buffer) 00 64
    ///        9) 1) (Empty Array)
    ///           2) (Integer) 123
    ///           3) \"Bulk String\"
    ///    11) (Null)
    /// 13) (Null)
    /// ```
    pub fn to_string_pretty(&self) -> String {
        match *self {
            Value::Null => "(Null)".to_string(),
            Value::NullArray => "(Null Array)".to_string(),
            Value::String(ref val) => val.to_string(),
            Value::Error(ref val) => format!("(Error) {}", val),
            Value::StaticError(ref val) => format!("(Error) {}", val),
            Value::Integer(ref val) => format!("(Integer) {}", val.to_string()),
            Value::Bulk(ref val) => format!("\"{}\"", val),
            Value::BufBulk(ref val) => {
                if val.is_empty() {
                    return "(Empty Buffer)".to_string();
                }
                let mut string = String::new();
                for u in val.iter().take(16) {
                    string.push_str(&format_to_hex_str(u));
                }
                if val.len() > 16 {
                    string.push_str(" ...");
                }
                format!("(Buffer) {}", &string[1..])
            }
            Value::Array(ref val) => format_array_to_str(val, 0),
        }
    }
}

fn format_to_hex_str(u: &u8) -> String {
    if *u >= 16 {
        format!(" {:x}", u)
    } else {
        format!(" 0{:x}", u)
    }
}

fn format_index_str(index: usize, num_len: usize) -> String {
    let mut string = index.to_string();
    let len = string.len();

    if num_len > len {
        let mut len = len;
        string.reserve(num_len - len);
        loop {
            string.insert(0, ' ');
            len += 1;
            if num_len == len {
                break;
            }
        }
    }
    format!("{}) ", string)
}

fn format_array_to_str(array: &[Value], min_index_len: usize) -> String {
    if array.is_empty() {
        return "(Empty Array)".to_string();
    }

    let mut string = String::new();
    let mut index_len = min_index_len;
    let len = array.len();
    let num_len = len.to_string().len();
    if num_len > index_len {
        index_len = num_len;
    }
    for (i, value) in array.iter().enumerate() {
        // first element don't need indent.
        let num_len = if i == 0 {
            index_len - min_index_len
        } else {
            index_len
        };
        string.push_str(&format_index_str(i + 1, num_len));
        match *value {
            Value::Array(ref sub) => string.push_str(&format_array_to_str(sub, index_len + 3)),
            _ => string.push_str(&value.to_string_pretty()),
        };
        if i + 1 < len {
            string.push('\n');
        }
    }
    string
}
