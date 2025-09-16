use crate::args::RuntimeArgs;
use crate::net::{connect, Stream};
use anyhow::{bail, Context, Result};
use firefly_types::serial;

pub fn cmd_exit(root_args: &RuntimeArgs) -> Result<()> {
    println!("⏳️ connecting...");
    let mut stream = connect(&root_args.port)?;
    stream.set_timeout(2);

    println!("⌛ exiting the running app...");
    let req = serial::Request::Exit;
    stream.send(&req).context("send request")?;

    for _ in 0..5 {
        let resp = stream.next()?;
        if matches!(resp, serial::Response::Ok) {
            println!("✅ exited");
            return Ok(());
        }
    }
    bail!("timed out waiting for response")
}

pub fn cmd_restart(root_args: &RuntimeArgs) -> Result<()> {
    println!("⏳️ connecting...");
    let mut stream = connect(&root_args.port)?;
    stream.set_timeout(2);

    let (author_id, app_id) = read_app_id(&mut *stream).context("fetch ID")?;

    println!("⌛ restarting {author_id}.{app_id}...");
    let req = serial::Request::Launch((author_id, app_id));
    stream.send(&req).context("send request")?;

    for _ in 0..5 {
        let resp = stream.next()?;
        if matches!(resp, serial::Response::Ok) {
            println!("✅ restarted");
            return Ok(());
        }
    }
    bail!("timed out waiting for response")
}

pub fn cmd_id(root_args: &RuntimeArgs) -> Result<()> {
    eprintln!("⏳️ connecting...");
    let mut stream = connect(&root_args.port)?;
    stream.set_timeout(2);
    let (author_id, app_id) = read_app_id(&mut *stream).context("fetch ID")?;
    eprintln!("✅ got the ID:");
    println!("{author_id}.{app_id}");
    Ok(())
}

pub fn read_app_id(stream: &mut dyn Stream) -> Result<(String, String)> {
    println!("⌛ fetching running app ID...");
    stream
        .send(&serial::Request::AppId)
        .context("send request")?;

    for _ in 0..5 {
        let resp = stream.next()?;
        if let serial::Response::AppID(id) = resp {
            return Ok(id);
        }
    }
    bail!("timed out waiting for response")
}
