use crate::args::{CheatArgs, RuntimeArgs};
use crate::config::Config;
use crate::net::connect;
use anyhow::{Context, Result, bail};
use firefly_types::serial;
use std::path::Path;

pub fn cmd_cheat(root_args: &RuntimeArgs, args: &CheatArgs) -> Result<()> {
    println!("⏳️ connecting...");
    let mut stream = connect(root_args)?;
    stream.set_timeout(2);

    {
        let cmd = parse_command(&args.command, &args.root)?;
        let val = parse_value(&args.value)?;
        let req = serial::Request::Cheat(cmd, val);
        println!("⌛ sending request...");
        stream.send(&req)?;
    }

    println!("⌛ waiting for response...");
    for _ in 0..5 {
        match stream.next() {
            Ok(serial::Response::Cheat(result)) => {
                println!("✅  response: {result}");
                return Ok(());
            }
            Ok(serial::Response::Log(log)) => println!("🪵 {log}"),
            Ok(_) => (),
            Err(err) => return Err(err),
        }
    }
    bail!("timed out waiting for response")
}

/// Parse a cheat command as either an integer or a cheat from firefly.toml.
fn parse_command(raw: &str, root: &Path) -> Result<i32> {
    if let Ok(n) = raw.parse::<i32>() {
        return Ok(n);
    }
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
            println!(
                "⚠️  interpreting the value as a character. Pass 'yes' or 'no' if you want to pass a boolean."
            );
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
