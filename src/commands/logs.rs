use crate::{args::LogsArgs, serial::SerialStream};
use anyhow::{Context, Result};
use firefly_types::serial::Response;
use std::time::Duration;

pub fn cmd_logs(args: &LogsArgs) -> Result<()> {
    let port = serialport::new(&args.port, args.baud_rate)
        .timeout(Duration::from_secs(3600))
        .open()
        .context("open the serial port")?;
    let mut stream = SerialStream::new(port);
    println!("listening...");
    loop {
        match stream.next()? {
            Response::Log(log) => println!("{log}"),
            Response::Cheat(val) => println!("cheat response: {val}"),
            _ => (),
        }
    }
}
