use crate::args::CheatArgs;
use crate::config::Config;
use crate::net::connect;
use anyhow::{bail, Context, Result};
use firefly_types::serial;
use firefly_types::Encode;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

pub fn cmd_cheat(args: &CheatArgs) -> Result<()> {
    println!("⏳️  connecting...");
    let mut stream = connect()?;

    {
        let mut buf = vec![0; 64];
        let cmd = parse_command(&args.command, &args.root)?;
        let val = parse_value(&args.value)?;
        let req = serial::Request::Cheat(cmd, val);
        let buf = req.encode(&mut buf).context("encode request")?;
        println!("⌛  sending request...");
        stream.write_all(buf).context("send request")?;
        stream.flush().context("flush request")?;
    }

    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    for _ in 0..5 {
        let mut buf = vec![0; 64];
        stream.read(&mut buf).context("read response")?;
        let resp = serial::Response::decode(&buf).context("decode response")?;
        if let serial::Response::Cheat(result) = resp {
            println!("✅  response: {result}");
            return Ok(());
        }
    }
    bail!("timed out waiting for response")
}

/// Parse a cheat command as either an integer or a cheat from firefly.toml.
fn parse_command(raw: &str, root: &Path) -> Result<i32> {
    if let Ok(n) = raw.parse::<i32>() {
        return Ok(n);
    };
    let config = Config::load(root.into(), root).context("load project config")?;
    let Some(cheats) = config.cheats else {
        bail!("firefly.toml doesn't have [cheats]")
    };
    let Some(n) = cheats.get(raw) else {
        bail!("command not found in [cheats]")
    };
    Ok(*n)
}

/// Parse cheat value as integer, character, or boolean.
fn parse_value(raw: &str) -> Result<i32> {
    if let Ok(n) = raw.parse::<i32>() {
        return Ok(n);
    }
    if raw == "true" || raw == "yes" || raw == "on" {
        return Ok(1);
    }
    if raw == "false" || raw == "no" || raw == "off" {
        return Ok(0);
    }
    if raw.len() == 1 {
        if raw == "y" || raw == "n" {
            println!("⚠️  interpreting the value as a character. Pass 'yes' or 'no' if you want to pass a boolean.");
        }
        let ch = raw.chars().next().unwrap();
        let n: u32 = ch.into();
        if let Ok(n) = i32::try_from(n) {
            return Ok(n);
        }
    }
    bail!("the value must be an integer, character, or boolean")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("a").unwrap(), 97);
        assert_eq!(parse_value("true").unwrap(), 1);
        assert_eq!(parse_value("false").unwrap(), 0);
        assert_eq!(parse_value("13").unwrap(), 13);
        assert_eq!(parse_value("-42").unwrap(), -42);
        assert!(parse_value("test").is_err());
    }
}
