// Little-endian

// https://stackoverflow.com/a/36676814/6438359
pub fn to_u32(arr: &[u8]) -> u32 {
    (arr[0] as u32) | (arr[1] as u32) << 8 | (arr[2] as u32) << 16 | (arr[3] as u32) << 24
}

pub fn from_u32(arr: &mut [u8], x: u32) {
    arr[0] = x as u8;
    arr[1] = (x >> 8) as u8;
    arr[2] = (x >> 16) as u8;
    arr[3] = (x >> 24) as u8;
}
