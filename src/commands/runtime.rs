use crate::args::RuntimeArgs;
use crate::net::{connect, Stream};
use anyhow::{bail, Context, Result};
use firefly_types::serial;

pub fn cmd_exit(root_args: &RuntimeArgs) -> Result<()> {
    println!("⏳️ connecting...");
    let mut stream = connect(root_args)?;
    stream.set_timeout(2);

    println!("⌛ exiting the running app...");
    let req = serial::Request::Exit;
    stream.send(&req).context("send request")?;
    wait_for_ok(stream)
}

pub fn cmd_restart(root_args: &RuntimeArgs) -> Result<()> {
    println!("⏳️ connecting...");
    let mut stream = connect(root_args)?;
    stream.set_timeout(2);

    let (author_id, app_id) = read_app_id(&mut *stream).context("fetch ID")?;
    println!("⌛ restarting {author_id}.{app_id}...");
    let req = serial::Request::Launch((author_id, app_id));
    stream.send(&req).context("send request")?;
    wait_for_ok(stream)
}

pub fn cmd_id(root_args: &RuntimeArgs) -> Result<()> {
    eprintln!("⏳️ connecting...");
    let mut stream = connect(root_args)?;
    stream.set_timeout(2);

    let (author_id, app_id) = read_app_id(&mut *stream).context("fetch ID")?;
    eprintln!("✅ got the ID:");
    println!("{author_id}.{app_id}");
    Ok(())
}

pub fn cmd_screenshot(root_args: &RuntimeArgs) -> Result<()> {
    eprintln!("⏳️ connecting...");
    let mut stream = connect(root_args)?;
    stream.set_timeout(2);

    println!("⌛ sending request...");
    let req = serial::Request::Screenshot;
    stream.send(&req).context("send request")?;
    wait_for_ok(stream)
}

fn read_app_id(stream: &mut dyn Stream) -> Result<(String, String)> {
    println!("⌛ fetching running app ID...");
    let req = serial::Request::AppId;
    stream.send(&req).context("send request")?;
    for _ in 0..5 {
        let resp = stream.next()?;
        if let serial::Response::AppID(id) = resp {
            return Ok(id);
        }
    }
    bail!("timed out waiting for response")
}

fn wait_for_ok(stream: Box<dyn Stream>) -> Result<()> {
    let mut stream = stream;
    for _ in 0..5 {
        let resp = stream.next()?;
        if matches!(resp, serial::Response::Ok) {
            println!("✅ done");
            return Ok(());
        }
    }
    bail!("timed out waiting for response")
}
