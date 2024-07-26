use crate::args::MonitorArgs;
use anyhow::Result;
use crossterm::terminal::ClearType;
use crossterm::{cursor, execute, style, terminal};
use std::io;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub fn cmd_monitor(_vfs: &Path, args: &MonitorArgs) -> Result<()> {
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    execute!(io::stdout(), cursor::Hide)?;
    let res = run_monitor(args);
    execute!(io::stdout(), terminal::LeaveAlternateScreen)?;
    res
}

fn run_monitor(args: &MonitorArgs) -> Result<()> {
    execute!(io::stdout(), cursor::MoveTo(0, 0))?;
    execute!(io::stdout(), style::Print("hello!!!"))?;
    sleep(Duration::from_secs(1));

    execute!(io::stdout(), terminal::Clear(ClearType::All))?;
    execute!(io::stdout(), cursor::MoveTo(0, 0))?;
    execute!(io::stdout(), style::Print("world"))?;
    sleep(Duration::from_secs(1));
    Ok(())
}
