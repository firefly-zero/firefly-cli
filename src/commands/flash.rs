use crate::{
    args::{FlashArgs, RuntimeArgs},
    fs::path_to_utf8,
};
use anyhow::{Context, Result, bail};
use std::{env::temp_dir, fs, path::Path, process::Command};

/// `ff flash`: Flash firmware into device or file.
pub fn cmd_flash(root_args: &RuntimeArgs, args: &FlashArgs) -> Result<()> {
    if let Some(port) = &root_args.port
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
        write_serial(root_args, serial).context("write serial number")?;
    }

    if is_source(args)? {
        println!("⏳️ flashing firmware from source...");
        flash_from_source(root_args, args)?;
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
fn write_serial(root_args: &RuntimeArgs, serial: u32) -> Result<()> {
    let serial_path = temp_dir().join("firefly-serial.bin");
    fs::write(serial_path, serial.to_le_bytes()).context("write serial number into temp file")?;
    let mut cmd_args: Vec<&str> = vec![
        "write-bin",
        "--skip-update-check",
        "--non-interactive",
        "--chip",
        "esp32s3",
        "0x10000",
        "/tmp/serial.txt",
    ];
    if let Some(port) = &root_args.port {
        cmd_args.push("--port");
        cmd_args.push(port);
    }
    let root = std::env::current_dir()?;
    exec_espflash(&root, &cmd_args)?;
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
fn flash_from_source(root_args: &RuntimeArgs, args: &FlashArgs) -> Result<()> {
    let root = if let Some(path) = &args.input {
        path
    } else {
        &std::env::current_dir().context("detect current dir")?
    };
    let mut shared_args = vec![
        "--skip-update-check",
        "--non-interactive",
        "--chip",
        "esp32s3",
    ];
    if let Some(port) = &root_args.port {
        shared_args.push("--port");
        shared_args.push(port);
    }

    // If output path is provided, save the image into the file.
    if let Some(output_path) = &args.output {
        // TODO: support saving as gz file
        let revision = format!("v{}", args.revision);
        let mut cmd_args = vec![
            "save-image",
            "--features",
            &revision,
            "--release",
            path_to_utf8(output_path)?,
        ];
        cmd_args.extend_from_slice(&shared_args);
        exec_espflash(root, &cmd_args).context("save image")?;
        return Ok(());
    }

    // Switch OTA to the factory slot.
    let partitions_path = root.join("partitions.csv");
    let partitions = path_to_utf8(&partitions_path)?;
    let mut cmd_args = vec!["erase-parts", "--partition-table", partitions, "otadata"];
    cmd_args.extend_from_slice(&shared_args);
    exec_espflash(root, &cmd_args).context("erase OTA partition")?;

    // Flash the image to the device.
    let revision = format!("v{}", args.revision);
    let mut cmd_args = vec![
        "flash",
        "--features",
        &revision,
        "--release",
        "--partition-table",
        partitions,
        "--target-app-partition",
        "factory",
    ];
    cmd_args.extend_from_slice(&shared_args);
    exec_espflash(root, &cmd_args).context("flash firmware")?;

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

fn exec_espflash(root: &Path, cmd_args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let mut cmd = cmd.arg("espflash").args(cmd_args).current_dir(root);

    // Set env vars from ~/export-esp.sh.
    if let Some(home) = std::env::home_dir() {
        let dotenv_path = home.join("export-esp.sh");
        if dotenv_path.is_file() {
            let dotenv_raw = fs::read_to_string(dotenv_path).context("read ~/export-esp.sh")?;
            let parts: Vec<_> = dotenv_raw.split('"').collect();
            if parts.len() == 5 {
                cmd = cmd.env("PATH", parts[1]);
                cmd = cmd.env("LIBCLANG_PATH", parts[3]);
            }
        }
    }

    cmd.status().context("run espflash")?;
    Ok(())
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
