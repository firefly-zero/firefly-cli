use crate::{args::LogsArgs, serial::read_cobs_frame};
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
            let (frame, rest) = read_cobs_frame(&buf);
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
