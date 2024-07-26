use crate::args::MonitorArgs;
use anyhow::{Context, Result};
use crossterm::terminal::ClearType;
use crossterm::{cursor, execute, style, terminal};
use firefly_types::serial;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;

static IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const TCP_PORT_MIN: u16 = 3210;
const TCP_PORT_MAX: u16 = 3217;
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
    execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print("connecting..."),
    )?;

    let addrs: Vec<_> = (TCP_PORT_MIN..=TCP_PORT_MAX)
        .map(|port| SocketAddr::new(IP, port))
        .collect();
    let mut stream = TcpStream::connect(&addrs[..]).context("connect to emulator")?;

    execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print("waiting for stats..."),
    )?;

    // enable stats collection
    {
        let mut buf = vec![0; 64];
        let req = serial::Request::Stats(true);
        let buf = req.encode(&mut buf).context("encode request")?;
        stream.write_all(&buf).context("send request")?;
        stream.flush().context("flush request")?;
    }

    let mut stats = Stats {
        update: None,
        render: None,
        cpu: None,
        mem: None,
    };
    loop {
        let mut buf = vec![0; 64];
        stream.read(&mut buf)?;
        let resp = serial::Response::decode(&buf)?;
        match resp {
            serial::Response::Cheat(_) => {}
            serial::Response::Fuel(cb, fuel) => match cb {
                serial::Callback::Boot => {}
                serial::Callback::Update => stats.update = Some(fuel),
                serial::Callback::Render => stats.render = Some(fuel),
                serial::Callback::RenderLine => {}
                serial::Callback::Cheat => {}
            },
            serial::Response::CPU(cpu) => {
                if cpu.total_ns > 0 {
                    stats.cpu = Some(cpu)
                }
            }
            serial::Response::Memory(mem) => stats.mem = Some(mem),
        };
        render_stats(&stats)?;
    }
}

fn render_stats(stats: &Stats) -> Result<()> {
    execute!(io::stdout(), terminal::Clear(ClearType::All))?;
    if let Some(cpu) = &stats.cpu {
        render_cpu(cpu).context("render cpu table")?;
    };
    if let Some(fuel) = &stats.update {
        render_fuel(7, "update", fuel).context("render fuel table")?;
    };
    if let Some(fuel) = &stats.render {
        render_fuel(14, "render", fuel).context("render fuel table")?;
    };
    if let Some(memory) = &stats.mem {
        render_memory(memory).context("render memory table")?;
    };
    Ok(())
}

fn render_cpu(cpu: &serial::CPU) -> anyhow::Result<()> {
    if cpu.total_ns == 0 {
        return Ok(());
    }
    let idle = cpu.total_ns.saturating_sub(cpu.busy_ns);
    execute!(
        io::stdout(),
        cursor::MoveTo(0, 1),
        // https://en.wikipedia.org/wiki/Box-drawing_characters
        style::Print("┌╴cpu╶───────────────┐"),
        cursor::MoveTo(0, 2),
        style::Print("│ lag"),
        cursor::MoveTo(COL1, 2),
        style::Print(&format_ns(cpu.lag_ns)),
        cursor::MoveTo(COL2, 2),
        style::Print(&format_ratio(cpu.lag_ns, cpu.total_ns)),
        cursor::MoveTo(0, 3),
        style::Print("│ busy"),
        cursor::MoveTo(COL1, 3),
        style::Print(&format_ns(cpu.busy_ns)),
        cursor::MoveTo(COL2, 3),
        style::Print(&format_ratio(cpu.busy_ns, cpu.total_ns)),
        cursor::MoveTo(0, 4),
        style::Print("│ idle"),
        cursor::MoveTo(COL1, 4),
        style::Print(&format_ns(idle)),
        cursor::MoveTo(COL2, 4),
        style::Print(&format_ratio(idle, cpu.total_ns)),
        cursor::MoveTo(RBORD, 2),
        style::Print("│"),
        cursor::MoveTo(RBORD, 3),
        style::Print("│"),
        cursor::MoveTo(RBORD, 4),
        style::Print("│"),
        cursor::MoveTo(0, 5),
        style::Print("└────────────────────┘"),
    )?;
    Ok(())
}

fn render_fuel(start: u16, name: &str, fuel: &serial::Fuel) -> anyhow::Result<()> {
    if fuel.calls == 0 {
        return Ok(());
    }
    execute!(
        io::stdout(),
        cursor::MoveTo(0, start),
        // https://en.wikipedia.org/wiki/Box-drawing_characters
        style::Print(format!("┌╴fuel: {name}╶──────┐")),
        cursor::MoveTo(0, start + 1),
        style::Print("│ min"),
        cursor::MoveTo(COL1, start + 1),
        style::Print(format_value(fuel.min)),
        cursor::MoveTo(0, start + 2),
        style::Print("│ max"),
        cursor::MoveTo(COL1, start + 2),
        style::Print(format_value(fuel.max)),
        cursor::MoveTo(0, start + 3),
        style::Print("│ mean"),
        cursor::MoveTo(COL1, start + 3),
        style::Print(format_value(fuel.mean)),
        cursor::MoveTo(0, start + 4),
        style::Print("│ stdev"),
        cursor::MoveTo(COL1, start + 4),
        style::Print(format_value(fuel.var.sqrt() as u32)),
        cursor::MoveTo(0, start + 4),
        cursor::MoveTo(RBORD, start + 1),
        style::Print("│"),
        cursor::MoveTo(RBORD, start + 2),
        style::Print("│"),
        cursor::MoveTo(RBORD, start + 3),
        style::Print("│"),
        cursor::MoveTo(RBORD, start + 4),
        style::Print("│"),
        cursor::MoveTo(0, start + 5),
        style::Print("└────────────────────┘"),
    )?;
    Ok(())
}

fn render_memory(memory: &serial::Memory) -> anyhow::Result<()> {
    let start = 20;
    if memory.pages == 0 {
        return Ok(());
    }
    execute!(
        io::stdout(),
        cursor::MoveTo(0, start),
        // https://en.wikipedia.org/wiki/Box-drawing_characters
        style::Print(format!("┌╴memory╶────────────┐")),
        cursor::MoveTo(0, start + 1),
        style::Print("│ min"),
        cursor::MoveTo(COL1, start + 1),
        style::Print(format_bytes(memory.last_one)),
        cursor::MoveTo(0, start + 2),
        style::Print("│ max"),
        cursor::MoveTo(COL1, start + 2),
        style::Print(format_bytes(memory.pages as u32 * 64 * KB)),
        cursor::MoveTo(RBORD, start + 1),
        style::Print("│"),
        cursor::MoveTo(RBORD, start + 2),
        style::Print("│"),
        cursor::MoveTo(0, start + 3),
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
    format!("{:>4} ns", ns)
}

fn format_ratio(n: u32, d: u32) -> String {
    if d == 0 {
        return "  0%".to_string();
    }
    let r = (n as f64 * 100.) / (d as f64);
    let r = r.round_ties_even() as u8;
    format!("{:>3}%", r)
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
