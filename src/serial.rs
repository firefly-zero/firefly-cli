// Given the binary stream so far, read the first COBS frame and return the rest of bytes.
pub fn read_cobs_frame(chunk: &[u8]) -> (Vec<u8>, &[u8]) {
    let max_len = chunk.len();
    let mut out_buf = vec![0; max_len];
    let mut dec = cobs::CobsDecoder::new(&mut out_buf);
    match dec.push(chunk) {
        Ok(Some((n_out, n_in))) => {
            let msg = Vec::from(&out_buf[..n_out]);
            (msg, &chunk[n_in..])
        }
        Ok(None) => (Vec::new(), chunk),
        Err(err) => match err {
            cobs::DecodeError::EmptyFrame => (Vec::new(), &[]),
            cobs::DecodeError::InvalidFrame { decoded_bytes } => {
                (Vec::new(), &chunk[decoded_bytes..])
            }
            cobs::DecodeError::TargetBufTooSmall => unreachable!(),
        },
    }
}
