use crate::args::LogsArgs;
use anyhow::{Context, Result};
use std::time::Duration;

pub fn cmd_logs(args: &LogsArgs) -> Result<()> {
    let mut port = serialport::new(&args.port, 115_200)
        .timeout(Duration::from_millis(10))
        .open()
        .context("open the serial port")?;
    let mut buf: Vec<u8> = vec![0; 32];
    println!("listening...");
    loop {
        match port.read(buf.as_mut_slice()) {
            Ok(n) => println!("{:?}", &buf[..n]),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::TimedOut {
                    continue;
                }
                Err(err)?;
            }
        };
    }
}
