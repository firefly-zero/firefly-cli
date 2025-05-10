use crate::args::LogsArgs;
use anyhow::{Context, Result};
use firefly_types::{serial::Response, Encode};
use std::time::Duration;

pub fn cmd_logs(args: &LogsArgs) -> Result<()> {
    let mut port = serialport::new(&args.port, args.baud_rate)
        .timeout(Duration::from_millis(10))
        .open()
        .context("open the serial port")?;
    let mut buf = Vec::new();
    println!("listening...");
    loop {
        let mut chunk = vec![0; 64];
        let n = match port.read(chunk.as_mut_slice()) {
            Ok(n) => n,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::TimedOut {
                    continue;
                }
                return Err(err).context("read from serial port");
            }
        };

        buf.extend_from_slice(&chunk[..n]);
        loop {
            let (frame, rest) = advance(&buf);
            buf = Vec::from(rest);
            if frame.is_empty() {
                break;
            }
            match Response::decode(&frame) {
                Ok(Response::Log(log)) => println!("{log}"),
                Ok(_) => (),
                Err(err) => println!("invalid message: {err}"),
            }
        }
    }
}

// Given the binary stream so far, read the first COBS frame and return the rest of bytes.
pub(super) fn advance(chunk: &[u8]) -> (Vec<u8>, &[u8]) {
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
