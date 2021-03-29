pub fn fill(a: &mut [u8], b: &[u8], start: usize) {
    for it in a.iter_mut().skip((start)).zip(b.iter()) {
        let (ai, bi) = it;
        *ai = *bi;
    }
}

pub(crate) fn read_up_to(
    file: &mut impl std::io::Read,
    mut buf: &mut [u8],
) -> Result<usize, std::io::Error> {
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