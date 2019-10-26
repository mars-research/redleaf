// Little-endian

// https://stackoverflow.com/a/36676814/6438359
pub fn to_u32(arr: &[u8; 4]) -> u32 {
    (arr[0] as u32) |
    (arr[1] as u32) << 8 |
    (arr[2] as u32) << 16 |
    (arr[3] as u32) << 24
}