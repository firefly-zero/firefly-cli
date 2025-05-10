use super::logs::advance;
use crate::args::MonitorArgs;
use crate::net::connect;
use anyhow::{Context, Result};
use crossterm::{cursor, event, execute, style, terminal};
use firefly_types::{serial, Encode};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

const COL1: u16 = 8;
const COL2: u16 = 16;
const RBORD: u16 = 21;
const KB: u32 = 1024;
const MB: u32 = 1024 * KB;

type Port = Box<dyn serialport::SerialPort>;

#[derive(Default)]
struct Stats {
    update: Option<serial::Fuel>,
    render: Option<serial::Fuel>,
    cpu: Option<serial::CPU>,
    mem: Option<serial::Memory>,
    log: Option<String>,
}

impl Stats {
    const fn is_default(&self) -> bool {
        self.update.is_none()
            && self.render.is_none()
            && self.cpu.is_none()
            && self.mem.is_none()
            && self.log.is_none()
    }
}

pub fn cmd_monitor(_vfs: &Path, args: &MonitorArgs) -> Result<()> {
    execute!(io::stdout(), terminal::EnterAlternateScreen).context("enter alt screen")?;
    execute!(io::stdout(), cursor::Hide).context("hide cursor")?;
    terminal::enable_raw_mode().context("enable raw mode")?;
    let res = if let Some(port) = &args.port {
        monitor_device(port, args)
    } else {
        monitor_emulator()
    };
    terminal::disable_raw_mode().context("disable raw mode")?;
    execute!(io::stdout(), terminal::LeaveAlternateScreen).context("leave alt screen")?;
    res
}

fn monitor_device(port: &str, args: &MonitorArgs) -> Result<()> {
    let mut port = connect_device(port, args)?;
    let mut stats = Stats::default();
    let mut buf = Vec::new();
    loop {
        if should_exit() {
            return Ok(());
        }
        buf = read_device(&mut port, buf, &mut stats)?;
        render_stats(&stats).context("render stats")?;
    }
}

/// Connect to running emulator using serial USB port (JTag-over-USB).
fn connect_device(port: &str, args: &MonitorArgs) -> Result<Port> {
    let mut port = serialport::new(port, args.baud_rate)
        .timeout(Duration::from_millis(10))
        .open()
        .context("open the serial port")?;

    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print("waiting for stats..."),
    )?;

    // enable stats collection
    {
        let req = serial::Request::Stats(true);
        let buf = req.encode_vec().context("encode request")?;
        port.write_all(&buf[..]).context("send request")?;
        port.flush().context("flush request")?;
    }

    Ok(port)
}

fn monitor_emulator() -> Result<()> {
    let mut stream = connect_emulator()?;
    let mut stats = Stats::default();
    loop {
        if should_exit() {
            return Ok(());
        }
        stream = read_emulator(stream, &mut stats)?;
        render_stats(&stats).context("render stats")?;
    }
}

/// Receive and parse one stats message from emulator.
fn read_emulator(mut stream: TcpStream, stats: &mut Stats) -> Result<TcpStream> {
    let mut buf = vec![0; 64];
    let size = stream.read(&mut buf).context("read response")?;
    if size == 0 {
        let stream = connect().context("reconnecting")?;
        return Ok(stream);
    }
    parse_stats(stats, &buf[..size])?;
    Ok(stream)
}

/// Receive and parse one stats message from device.
fn read_device(port: &mut Port, mut buf: Vec<u8>, stats: &mut Stats) -> Result<Vec<u8>> {
    let mut chunk = vec![0; 64];
    let n = match port.read(chunk.as_mut_slice()) {
        Ok(n) => n,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::TimedOut {
                return Ok(buf);
            }
            return Err(err).context("read from serial port");
        }
    };

    buf.extend_from_slice(&chunk[..n]);
    loop {
        let (frame, rest) = advance(&buf);
        buf = Vec::from(rest);
        if frame.is_empty() {
            break;
        }
        parse_stats(stats, &frame)?;
    }
    Ok(buf)
}

/// Parse raw stats message using postcard. Does NOT handle COBS frames.
fn parse_stats(stats: &mut Stats, buf: &[u8]) -> Result<()> {
    let resp = serial::Response::decode(buf).context("decode response")?;
    match resp {
        serial::Response::Cheat(_) => {}
        serial::Response::Log(log) => {
            let now = chrono::Local::now().format("%H:%M:%S");
            let log = format!("[{now}] {log}");
            stats.log = Some(log);
        }
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
    Ok(())
}

/// Connect to a running emulator and enable stats.
fn connect_emulator() -> Result<TcpStream, anyhow::Error> {
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
        let req = serial::Request::Stats(true);
        let buf = req.encode_vec().context("encode request")?;
        stream.write_all(&buf[..]).context("send request")?;
        stream.flush().context("flush request")?;
    }

    Ok(stream)
}

/// Check if the `Q` or `Esc` button is pressed.
fn should_exit() -> bool {
    let timeout = Duration::from_millis(0);
    while event::poll(timeout).unwrap_or_default() {
        let Ok(event) = event::read() else {
            continue;
        };
        let event::Event::Key(event) = event else {
            continue;
        };
        if event.kind != event::KeyEventKind::Press {
            continue;
        }
        if event.code == event::KeyCode::Char('q') {
            return true;
        }
        if event.code == event::KeyCode::Char('c') {
            return true;
        }
        if event.code == event::KeyCode::Esc {
            return true;
        }
    }
    false
}

/// Display stats in the terminal.
fn render_stats(stats: &Stats) -> Result<()> {
    if stats.is_default() {
        return Ok(());
    }
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
    if let Some(log) = &stats.log {
        render_log(log).context("render logs")?;
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
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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

fn render_log(log: &str) -> anyhow::Result<()> {
    execute!(io::stdout(), cursor::MoveTo(3, 13), style::Print(log),)?;
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
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let r = u8::try_from(r as u64).unwrap_or(255);
    if r == 0 && n > 0 {
        return "  1%".to_string();
    }
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
