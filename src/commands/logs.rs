use crate::args::LogsArgs;
use anyhow::{Context, Result};
use firefly_types::{serial::Response, Encode};
use std::time::Duration;

pub fn cmd_logs(args: &LogsArgs) -> Result<()> {
    let mut port = serialport::new(&args.port, 9600)
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
            let response = Response::decode(&frame).context("decode message")?;
            let Response::Log(log) = response else {
                continue;
            };
            println!("{log}");
        }
    }
}

// Given the binary stream so far, read the first COBS frame and return the rest of bytes.
fn advance(chunk: &[u8]) -> (Vec<u8>, &[u8]) {
    // Skip the partial frame: all bytes before the separator.
    let maybe = chunk.iter().enumerate().find(|(_, b)| **b == 0x00);
    let Some((start, _)) = maybe else {
        return (Vec::new(), chunk);
    };
    let chunk = &chunk[start..];

    let max_len = chunk.len();
    let mut out_buf = vec![0; max_len];
    let mut dec = cobs::CobsDecoder::new(&mut out_buf);
    match dec.push(&chunk[1..]) {
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
