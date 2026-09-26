use std::{env::temp_dir, fs, process::Command};

use crate::{args::FlashArgs, fs::path_to_utf8};
use anyhow::{Context, Result, bail};

/// `ff flash`: Flash firmware into device or file.
pub fn cmd_flash(args: &FlashArgs) -> Result<()> {
    if let Some(port) = &args.port
        && !port.starts_with("/dev/tty")
    {
        bail!("invalid --port");
    }
    if let Some(output) = &args.output
        && output.starts_with("/dev/")
    {
        bail!("invalid --output");
    }

    // Install cargo-espflash.
    if !espflash_installed() {
        if !cargo_installed() {
            bail!("cargo is not installed");
        }
        println!("⏳️ installing cargo-espflash...");
        Command::new("cargo")
            .args(["install", "cargo-espflash"])
            .output()?;
    }

    // If serial number is provided, write it to the device.
    if let Some(serial) = args.serial {
        println!("⏳️ writing serial number...");
        write_serial(args, serial).context("write serial number")?;
    }

    if is_source(args)? {
        println!("⏳️ flashing firmware from source...");
        flash_from_source(args)?;
    } else {
        // TODO: support installing from file.
        // TODO: support downloading and installing a release.
        bail!("firmware can only be built from source")
    }

    // TODO: monitor
    println!("✅ flashed");
    Ok(())
}

/// Write serial number into flash memory of the device.
fn write_serial(args: &FlashArgs, serial: u32) -> Result<()> {
    let serial_path = temp_dir().join("firefly-serial.bin");
    fs::write(serial_path, serial.to_le_bytes()).context("write serial number into temp file")?;
    let mut cmd_args: Vec<&str> = vec![
        "espflash",
        "write-bin",
        "--skip-update-check",
        "--non-interactive",
        "--chip",
        "esp32s3",
        "0x10000",
        "/tmp/serial.txt",
    ];
    if let Some(port) = &args.port {
        cmd_args.push("--port");
        cmd_args.push(port);
    }
    Command::new("cargo").args(&cmd_args).output()?;
    Ok(())
}

/// Check if the --input (or the current dir) is the firefly-main or firefly-io source code.
fn is_source(args: &FlashArgs) -> Result<bool> {
    let root = if let Some(path) = &args.input {
        path
    } else {
        &std::env::current_dir().context("detect current dir")?
    };
    if root.is_file() {
        return Ok(false);
    }
    let config_path = root.join("Cargo.toml");
    if !config_path.is_file() {
        return Ok(false);
    }
    let config = fs::read_to_string(config_path).context("read Cargo.toml")?;
    Ok(config.contains(r#"name = "firefly-main""#) || config.contains(r#"name = "firefly-io""#))
}

/// Build firmware from source and flash it to the device.
fn flash_from_source(args: &FlashArgs) -> Result<()> {
    // If output path is provided, save the image into the file.
    if let Some(output_path) = &args.output {
        // TODO: support saving as gz file
        let revision = format!("v{}", args.revision);
        let mut cmd_args = vec![
            "espflash",
            "save-image",
            "--skip-update-check",
            "--non-interactive",
            "--features",
            &revision,
            "--chip",
            "esp32s3",
            "--release",
            path_to_utf8(output_path)?,
        ];
        if let Some(port) = &args.port {
            cmd_args.push("--port");
            cmd_args.push(port);
        }
        Command::new("cargo").args(cmd_args).output()?;
        return Ok(());
    }

    // Switch OTA to the factory slot.
    let cmd_args = [
        "espflash",
        "erase-parts",
        "--partition-table",
        "partitions.csv",
        "otadata",
    ];
    Command::new("cargo").args(cmd_args).output()?;

    // Flash the image to the device.
    let revision = format!("v{}", args.revision);
    let mut cmd_args = vec![
        "espflash",
        "flash",
        "--skip-update-check",
        "--non-interactive",
        "--features",
        &revision,
        "--chip",
        "esp32s3",
        "--release",
        "--partition-table",
        "partitions.csv",
        "--target-app-partition",
        "factory",
    ];
    if let Some(port) = &args.port {
        cmd_args.push("--port");
        cmd_args.push(port);
    }
    Command::new("cargo").args(cmd_args).output()?;

    Ok(())
}

fn cargo_installed() -> bool {
    let output = Command::new("cargo").arg("version").output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success()
}

fn espflash_installed() -> bool {
    let output = Command::new("cargo-espflash").args(["--version"]).output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success()
}

// # https://taskfile.dev
// version: "3"
// dotenv:
//   - ~/export-esp.sh

// vars:
//   IMAGE: ../../apps/firefly-updates/firefly-main

//   monitor:
//     cmds:
//       - task: install-espflash
//       - cargo espflash monitor {{.CLI_ARGS}}
