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
        execute!(
            io::stdout(),
            cursor::MoveTo(1, 1),
            style::Print("lag"),
            cursor::MoveTo(8, 1),
            style::Print(&format_ns(cpu.lag_ns)),
            cursor::MoveTo(16, 1),
            style::Print(&format_ratio(cpu.lag_ns, cpu.total_ns)),
            cursor::MoveTo(1, 2),
            style::Print("busy"),
            cursor::MoveTo(8, 2),
            style::Print(&format_ns(cpu.busy_ns)),
            cursor::MoveTo(16, 2),
            style::Print(&format_ratio(cpu.busy_ns, cpu.total_ns)),
            cursor::MoveTo(1, 3),
            style::Print("total"),
            cursor::MoveTo(8, 3),
            style::Print(&format_ns(cpu.total_ns)),
        )?;
    };
    Ok(())
}

fn format_ns(ns: u32) -> String {
    if ns > 1_000_000_000 {
        return format!("{}s", ns / 1_000_000_000);
    }
    if ns > 1_000_000 {
        return format!("{}ms", ns / 1_000_000);
    }
    if ns > 1_000 {
        return format!("{}μs", ns / 1_000);
    }
    format!("{}ns", ns)
}

fn format_ratio(n: u32, d: u32) -> String {
    format!("{}%", n as u64 * 100 / d as u64)
}
