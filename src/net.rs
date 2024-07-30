use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread::sleep;
use std::time::Duration;

static IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const TCP_PORT_MIN: u16 = 3210;
const TCP_PORT_MAX: u16 = 3217;

/// Connect to a running emulator.
pub fn connect() -> Result<TcpStream> {
    let addrs: Vec<_> = (TCP_PORT_MIN..=TCP_PORT_MAX)
        .map(|port| SocketAddr::new(IP, port))
        .collect();
    let mut maybe_stream = TcpStream::connect(&addrs[..]);
    if maybe_stream.is_err() {
        sleep(Duration::from_secs(1));
        maybe_stream = TcpStream::connect(&addrs[..]);
    };
    let stream = maybe_stream.context("connect to emulator")?;
    Ok(stream)
}
