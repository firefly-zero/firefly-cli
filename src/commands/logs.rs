use crate::args::LogsArgs;
use crate::net::connect;
use anyhow::{Context, Result};
use crossterm::cursor::MoveToColumn;
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::{Clear, ClearType};
use firefly_types::serial::Response;
use std::io::{stdout, Write};

pub fn cmd_logs(args: &LogsArgs) -> Result<()> {
    let port = Some(args.port.to_string());
    let mut stream = connect(&port).context("open the serial port")?;
    println!("listening...");
    let mut prev_time = chrono::Local::now(); // when the previous record was received
    let mut prev_text = String::new(); // the text of the previous log record
    let mut repeats = 1; // how many times in a row we received the same log record
    let mut use_blue = true; // should the log record time be blue or magenta
    loop {
        let msg = stream.next()?;
        let now = chrono::Local::now();
        let now_str = now.format("%H:%M:%S").to_string();
        // When a lot of time has passed since the last log record,
        // switch the color of the current time.
        // The color switch makes for an easy visual grouping of log records
        // coming close to each other in time.
        if now - prev_time >= chrono::Duration::seconds(4) {
            use_blue = !use_blue;
        }
        prev_time = now;
        let now = if use_blue {
            now_str.blue()
        } else {
            now_str.magenta()
        };
        match msg {
            Response::Log(mut log) => {
                if prev_text == log {
                    _ = execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0));
                    repeats += 1;
                } else {
                    println!();
                    repeats = 1;
                }
                prev_text.clone_from(&log);
                if log.starts_with("ERROR(") {
                    log = log.red().to_string();
                }
                print!("{now} {log}");
                if repeats > 1 {
                    print!("{}", format!(" x{repeats}").cyan());
                }
                _ = stdout().flush();
            }
            Response::Cheat(val) => {
                println!("{now} cheat response: {val}");
            }
            _ => (),
        }
    }
}
