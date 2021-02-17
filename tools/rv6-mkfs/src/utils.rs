unsafe fn to_bytes<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        std::mem::size_of::<T>(),
    )
}

pub fn zero(buffer: &mut [u8]) {
    for i in &buffer { 
        *i = 0 
    }
}

// unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
//     ::std::slice::from_raw_parts(
//         (p as *const T) as *const u8,
//         ::std::mem::size_of::<T>(),
//     )
// }

// fn main() {
//     struct MyStruct {
//         id: u8,
//         data: [u8; 1024],
//     }
//     let my_struct = MyStruct { id: 0, data: [1; 1024] };
//     let bytes: &[u8] = unsafe { any_as_u8_slice(&my_struct) };
//     // tcp_stream.write(bytes);
//     println!("{:?}", bytes);
// }
