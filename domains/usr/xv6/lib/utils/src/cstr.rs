use alloc::string::{FromUtf8Error, String};

// Convert a null-terminated utf8 byte array to String
pub fn to_string(cstr: &[u8]) -> Result<String, FromUtf8Error> {
    let slice_till_null = &cstr[..cstr.iter().position(|&c| c == 0).unwrap_or(cstr.len())];
    String::from_utf8(slice_till_null.to_vec())
}
