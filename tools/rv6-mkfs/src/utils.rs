unsafe fn to_bytes<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        std::mem::size_of::<T>(),
    )
}

pub fn zero(buffer: &mut [u8]) {
    for i in buffer.iter_mut() {
        *i = 0 
    }
}

pub fn u32_as_u8_mut<'a>(src: &'a mut [u32]) -> &'a mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(src.as_mut_ptr() as *mut u8,
                                        src.len() * 4)
    }

    // let v: Vec<u8> = Vec::new();
    // v.reserve(src.len() * 4);

    // for num in src.iter() {
    //     v.extend_from_slice(&num.to_le_bytes());
    // }

    // v
}

#[no_mangle]
pub unsafe extern fn memcpy(dest: *mut u8, src: *const u8,
                            n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *((dest as usize + i) as *mut u8) = *((src as usize + i) as *const u8);
        i += 1;
    }

    dest
}

pub fn fill(a: &mut [u8], b: &[u8], start: usize) {
    for it in a.iter_mut().skip((start)).zip(b.iter()) {
        let (ai, bi) = it;
        *ai = *bi;
    }
}

pub(crate) fn read_up_to(file: &mut impl std::io::Read, mut buf: &mut [u8]) -> Result<usize, std::io::Error> {
    let buf_len = buf.len();

    while !buf.is_empty() {
        match file.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(buf_len - buf.len())
}

pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::std::mem::size_of::<T>(),
    )
}
