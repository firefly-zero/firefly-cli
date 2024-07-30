use crate::args::MonitorArgs;
use crate::net::connect;
use anyhow::{Context, Result};
use crossterm::{cursor, execute, style, terminal};
use firefly_types::serial;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;

const COL1: u16 = 8;
const COL2: u16 = 16;
const RBORD: u16 = 21;
const KB: u32 = 1024;
const MB: u32 = 1024 * KB;

struct Stats {
    update: Option<serial::Fuel>,
    render: Option<serial::Fuel>,
    cpu: Option<serial::CPU>,
    mem: Option<serial::Memory>,
}

pub fn cmd_monitor(_vfs: &Path, args: &MonitorArgs) -> Result<()> {
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    execute!(io::stdout(), cursor::Hide)?;
    let res = run_monitor(args);
    execute!(io::stdout(), terminal::LeaveAlternateScreen)?;
    res
}

fn run_monitor(_args: &MonitorArgs) -> Result<()> {
    let mut stream = connect_verbose()?;
    let mut stats = Stats {
        update: None,
        render: None,
        cpu: None,
        mem: None,
    };
    loop {
        let mut buf = vec![0; 64];
        let size = stream.read(&mut buf).context("read response")?;
        if size == 0 {
            stream = connect().context("reconnecting")?;
            continue;
        }
        let resp = serial::Response::decode(&buf[..size]).context("decode response")?;
        match resp {
            serial::Response::Cheat(_) => {}
            serial::Response::Fuel(cb, fuel) => {
                use serial::Callback::*;
                match cb {
                    Update => stats.update = Some(fuel),
                    Render => stats.render = Some(fuel),
                    RenderLine | Cheat | Boot => {}
                }
            }
            serial::Response::CPU(cpu) => {
                if cpu.total_ns > 0 {
                    stats.cpu = Some(cpu);
                }
            }
            serial::Response::Memory(mem) => stats.mem = Some(mem),
        };
        render_stats(&stats).context("render stats")?;
    }
}

/// Connect to a running emulator and enable stats.
fn connect_verbose() -> Result<TcpStream, anyhow::Error> {
    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print("connecting..."),
    )?;
    let mut stream = connect()?;

    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print("waiting for stats..."),
    )?;

    // enable stats collection
    {
        let mut buf = vec![0; 64];
        let req = serial::Request::Stats(true);
        let buf = req.encode(&mut buf).context("encode request")?;
        stream.write_all(buf).context("send request")?;
        stream.flush().context("flush request")?;
    }

    Ok(stream)
}

fn render_stats(stats: &Stats) -> Result<()> {
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
    if let Some(cpu) = &stats.cpu {
        render_cpu(cpu).context("render cpu table")?;
    };
    if let Some(fuel) = &stats.update {
        render_fuel(1, 7, "update", fuel).context("render fuel table")?;
    };
    if let Some(fuel) = &stats.render {
        render_fuel(24, 7, "render", fuel).context("render fuel table")?;
    };
    if let Some(memory) = &stats.mem {
        render_memory(memory).context("render memory table")?;
    };
    Ok(())
}

fn render_cpu(cpu: &serial::CPU) -> anyhow::Result<()> {
    const X: u16 = 1;
    const Y: u16 = 1;
    if cpu.total_ns == 0 {
        return Ok(());
    }
    let idle = cpu.total_ns.saturating_sub(cpu.busy_ns);
    execute!(
        io::stdout(),
        cursor::MoveTo(X, Y),
        // https://en.wikipedia.org/wiki/Box-drawing_characters
        style::Print("┌╴cpu╶───────────────┐"),
        cursor::MoveTo(X, Y + 1),
        style::Print("│ lag"),
        cursor::MoveTo(X + COL1, Y + 1),
        style::Print(&format_ns(cpu.lag_ns)),
        cursor::MoveTo(X + COL2, Y + 1),
        style::Print(&format_ratio(cpu.lag_ns, cpu.total_ns)),
        cursor::MoveTo(X, Y + 2),
        style::Print("│ busy"),
        cursor::MoveTo(X + COL1, Y + 2),
        style::Print(&format_ns(cpu.busy_ns)),
        cursor::MoveTo(X + COL2, Y + 2),
        style::Print(&format_ratio(cpu.busy_ns, cpu.total_ns)),
        cursor::MoveTo(X, Y + 3),
        style::Print("│ idle"),
        cursor::MoveTo(X + COL1, Y + 3),
        style::Print(&format_ns(idle)),
        cursor::MoveTo(X + COL2, Y + 3),
        style::Print(&format_ratio(idle, cpu.total_ns)),
        cursor::MoveTo(X + RBORD, Y + 1),
        style::Print("│"),
        cursor::MoveTo(X + RBORD, Y + 2),
        style::Print("│"),
        cursor::MoveTo(X + RBORD, Y + 3),
        style::Print("│"),
        cursor::MoveTo(X, Y + 4),
        style::Print("└────────────────────┘"),
    )?;
    Ok(())
}

fn render_fuel(x: u16, y: u16, name: &str, fuel: &serial::Fuel) -> anyhow::Result<()> {
    if fuel.calls == 0 {
        return Ok(());
    }
    execute!(
        io::stdout(),
        cursor::MoveTo(x, y),
        // https://en.wikipedia.org/wiki/Box-drawing_characters
        style::Print(format!("┌╴fuel: {name}╶──────┐")),
        cursor::MoveTo(x, y + 1),
        style::Print("│ min"),
        cursor::MoveTo(x + COL1, y + 1),
        style::Print(format_value(fuel.min)),
        cursor::MoveTo(x, y + 2),
        style::Print("│ max"),
        cursor::MoveTo(x + COL1, y + 2),
        style::Print(format_value(fuel.max)),
        cursor::MoveTo(x, y + 3),
        style::Print("│ mean"),
        cursor::MoveTo(x + COL1, y + 3),
        style::Print(format_value(fuel.mean)),
        cursor::MoveTo(x, y + 4),
        style::Print("│ stdev"),
        cursor::MoveTo(x + COL1, y + 4),
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        style::Print(format_value(fuel.var.sqrt() as u32)),
        cursor::MoveTo(x, y + 4),
        cursor::MoveTo(x + RBORD, y + 1),
        style::Print("│"),
        cursor::MoveTo(x + RBORD, y + 2),
        style::Print("│"),
        cursor::MoveTo(x + RBORD, y + 3),
        style::Print("│"),
        cursor::MoveTo(x + RBORD, y + 4),
        style::Print("│"),
        cursor::MoveTo(x, y + 5),
        style::Print("└────────────────────┘"),
    )?;
    Ok(())
}

fn render_memory(memory: &serial::Memory) -> anyhow::Result<()> {
    const X: u16 = 24;
    const Y: u16 = 1;
    if memory.pages == 0 {
        return Ok(());
    }
    execute!(
        io::stdout(),
        cursor::MoveTo(X, Y),
        // https://en.wikipedia.org/wiki/Box-drawing_characters
        style::Print("┌╴memory╶────────────┐"),
        cursor::MoveTo(X, Y + 1),
        style::Print("│ floor"),
        cursor::MoveTo(X + COL1, Y + 1),
        style::Print(format_bytes(memory.last_one)),
        cursor::MoveTo(X, Y + 2),
        style::Print("│ ceil"),
        cursor::MoveTo(X + COL1, Y + 2),
        style::Print(format_bytes(u32::from(memory.pages) * 64 * KB)),
        cursor::MoveTo(X + COL2, Y + 2),
        style::Print(format!("{}p", memory.pages)),
        cursor::MoveTo(X + RBORD, Y + 1),
        style::Print("│"),
        cursor::MoveTo(X + RBORD, Y + 2),
        style::Print("│"),
        cursor::MoveTo(X, Y + 3),
        style::Print("└────────────────────┘"),
    )?;
    Ok(())
}

fn format_ns(ns: u32) -> String {
    if ns > 10_000_000 {
        return format!("{:>4} ms", ns / 1_000_000);
    }
    if ns > 10_000 {
        return format!("{:>4} μs", ns / 1_000);
    }
    format!("{ns:>4} ns")
}

fn format_ratio(n: u32, d: u32) -> String {
    if d == 0 {
        return "  0%".to_string();
    }
    let r = f64::from(n) * 100. / f64::from(d);
    let r = r.round_ties_even();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let r = u8::try_from(r as u64).unwrap_or(255);
    format!("{r:>3}%")
}

fn format_value(x: u32) -> String {
    if x > 10_000 {
        return format!("{:>3}k", x / 1000);
    }
    format!("{x:>4}")
}

fn format_bytes(x: u32) -> String {
    if x > 4 * MB {
        return format!("{:>3} MB", x / MB);
    }
    if x > KB {
        return format!("{:>3} KB", x / KB);
    }
    format!("{x:>4}")
}
