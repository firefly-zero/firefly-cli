use crate::{args::LogsArgs, serial::SerialStream};
use anyhow::{Context, Result};
use crossterm::style::Stylize;
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
        let msg = stream.next()?;
        let now = chrono::Local::now();
        let now = now.format("%H:%M:%S").to_string().blue();
        match msg {
            Response::Log(log) => println!("{now} {log}"),
            Response::Cheat(val) => println!("{now} cheat response: {val}"),
            _ => (),
        }
    }
}
