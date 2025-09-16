use crate::args::{RestartArgs, RuntimeArgs};
use crate::net::connect;
use crate::serial::SerialStream;
use anyhow::{bail, Context, Result};
use firefly_types::{serial, Encode};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn cmd_restart(root_args: &RuntimeArgs, _: &RestartArgs) -> Result<()> {
    if let Some(port) = &root_args.port {
        restart_device(root_args, port)
    } else {
        restart_emulator()
    }
}

/// Restart app on desktop emulator.
pub fn restart_emulator() -> Result<()> {
    println!("⏳️ connecting...");
    let mut stream = connect()?;
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;

    println!("⌛ fetching running app ID...");
    let (author_id, app_id) = read_app_id_emulator(&mut stream).context("fetch ID")?;

    println!("⌛ restarting {author_id}.{app_id}...");
    let req = serial::Request::Launch((author_id, app_id));
    let buf = req.encode_vec().context("encode request")?;
    stream.write_all(&buf).context("send request")?;
    stream.flush().context("flush request")?;

    for _ in 0..5 {
        let mut buf = vec![0; 64];
        stream.read(&mut buf).context("read response")?;
        let resp = serial::Response::decode(&buf).context("decode response")?;
        if matches!(resp, serial::Response::Ok) {
            println!("✅ restarted");
            return Ok(());
        }
    }
    bail!("timed out waiting for response")
}

pub fn read_app_id_emulator(stream: &mut TcpStream) -> Result<(String, String)> {
    let req = serial::Request::AppId;
    let buf = req.encode_vec().context("encode request")?;
    stream.write_all(&buf).context("send request")?;
    stream.flush().context("flush request")?;

    for _ in 0..5 {
        let mut buf = vec![0; 64];
        stream.read(&mut buf).context("read response")?;
        let resp = serial::Response::decode(&buf).context("decode response")?;
        if let serial::Response::AppID(id) = resp {
            return Ok(id);
        }
    }
    bail!("timed out waiting for response")
}

/// Restart app on the connected device.
pub fn restart_device(args: &RuntimeArgs, port: &str) -> Result<()> {
    println!("⏳️ connecting...");
    let port = serialport::new(port, args.baud_rate)
        .timeout(Duration::from_secs(5))
        .open()
        .context("open the serial port")?;
    let mut stream = SerialStream::new(port);

    todo!()
}
