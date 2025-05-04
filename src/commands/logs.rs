use crate::args::LogsArgs;
use anyhow::{Context, Result};
use std::time::Duration;

pub fn cmd_logs(args: &LogsArgs) -> Result<()> {
    let mut port = serialport::new(&args.port, 9600)
        .timeout(Duration::from_millis(10))
        .open()
        .context("open the serial port")?;
    let mut buf: Vec<u8> = vec![0; 32];
    println!("listening...");
    loop {
        match port.read(buf.as_mut_slice()) {
            Ok(n) => {
                let bytes = &buf[..n];
                match std::str::from_utf8(bytes) {
                    Ok(s) => print!("{s}"),
                    Err(_) => print!("{bytes:?}"),
                };
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::TimedOut {
                    continue;
                }
                Err(err)?;
            }
        };
    }
}
