use crate::args::EmulatorArgs;
use crate::langs::check_output;
use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn cmd_emulator(args: &EmulatorArgs) -> Result<()> {
    let executed_dev = run_dev(args)?;
    if executed_dev {
        return Ok(());
    }
    let bins = [
        "./firefly_emulator",
        "./firefly_emulator.exe",
        "firefly_emulator",
        "firefly_emulator.exe",
        "firefly-emulator",
        "firefly-emulator.exe",
    ];
    for bin in bins {
        if binary_exists(bin) {
            println!("running {bin}...");
            let output = Command::new(bin).args(&args.args).output()?;
            check_output(&output).context("run emulator")?;
        }
    }
    bail!("emulator not installed");
}

fn run_dev(args: &EmulatorArgs) -> Result<bool> {
    // Check common places where firefly repo might be clonned.
    // If found, run the dev version using cargo.
    let Some(base_dirs) = directories::BaseDirs::new() else {
        return Ok(false);
    };
    if !binary_exists("cargo") {
        return Ok(false);
    }
    let home = base_dirs.home_dir();
    let paths = [
        home.join("Documents").join("firefly"),
        home.join("ff").join("firefly"),
        home.join("github").join("firefly"),
        home.join("firefly"),
    ];
    for dir_path in paths {
        let cargo_path = dir_path.join("Cargo.toml");
        if !cargo_path.is_file() {
            continue;
        }
        println!("running dev version from {}...", dir_path.to_str().unwrap());
        let output = Command::new("cargo")
            .arg("run")
            .arg("--")
            .args(&args.args)
            .current_dir(dir_path)
            .output()?;
        check_output(&output).context("run emulator")?;
        return Ok(true);
    }
    Ok(false)
}

fn binary_exists(bin: &str) -> bool {
    let output = Command::new(bin).arg("--help").output();
    if let Ok(output) = output {
        if output.status.success() {
            return true;
        }
    }
    false
}
