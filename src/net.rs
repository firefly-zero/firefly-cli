use crate::serial::*;
use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread::sleep;
use std::time::Duration;

static IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const TCP_PORT_MIN: u16 = 3210;
const TCP_PORT_MAX: u16 = 3217;

#[expect(clippy::ref_option)]
pub fn connect(port: &Option<String>) -> Result<Box<dyn Stream>> {
    let stream: Box<dyn Stream> = if let Some(port) = port {
        Box::new(connect_device(port)?)
    } else {
        Box::new(connect_emulator()?)
    };
    Ok(stream)
}

fn connect_device(port: &str) -> Result<SerialStream> {
    let baud_rate = 115_200;
    let port = serialport::new(port, baud_rate)
        .open()
        .context("open the serial port")?;
    Ok(SerialStream::new(port))
}

/// Connect to a running emulator.
fn connect_emulator() -> Result<TcpStream> {
    let addrs: Vec<_> = (TCP_PORT_MIN..=TCP_PORT_MAX)
        .map(|port| SocketAddr::new(IP, port))
        .collect();
    let mut maybe_stream = TcpStream::connect(&addrs[..]);
    if maybe_stream.is_err() {
        sleep(Duration::from_secs(1));
        maybe_stream = TcpStream::connect(&addrs[..]);
    }
    let stream = maybe_stream.context("connect to emulator")?;
    Ok(stream)
}
