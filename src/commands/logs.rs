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
    let mut prev_log = chrono::Local::now();
    let mut use_blue = true;
    loop {
        let msg = stream.next()?;
        let now = chrono::Local::now();
        let now_str = now.format("%H:%M:%S").to_string();
        // When a lot of time has passed since the last log record,
        // switch the color of the current time.
        // The color switch makes for an easy visual grouping of log records
        // coming close to each other in time.
        if now - prev_log >= chrono::Duration::seconds(4) {
            use_blue = !use_blue;
        }
        prev_log = now;
        let now = if use_blue {
            now_str.blue()
        } else {
            now_str.magenta()
        };
        match msg {
            Response::Log(mut log) => {
                if log.starts_with("ERROR(") {
                    log = log.red().to_string();
                }
                println!("{now} {log}");
            }
            Response::Cheat(val) => {
                println!("{now} cheat response: {val}");
            }
            _ => (),
        }
    }
}
