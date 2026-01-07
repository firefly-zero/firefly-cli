use crate::args::EmulatorArgs;
use crate::langs::check_output;
use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use tar::Archive;

pub fn cmd_emulator(vfs: &Path, args: &EmulatorArgs) -> Result<()> {
    let executed_dev = run_dev(args)?;
    if executed_dev {
        return Ok(());
    }
    let executed_bin = run_bin(args)?;
    if executed_bin {
        return Ok(());
    }
    run_embedded(vfs, args)
}

fn run_dev(args: &EmulatorArgs) -> Result<bool> {
    // Check common places where firefly repo might be cloned.
    // If found, run the dev version using cargo.
    let Some(home) = std::env::home_dir() else {
        return Ok(false);
    };
    if !binary_exists("cargo") {
        return Ok(false);
    }
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
        println!(
            "⌛ running dev version from {}...",
            dir_path.to_str().unwrap()
        );
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

fn run_bin(args: &EmulatorArgs) -> Result<bool> {
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
            println!("⌛ running {bin}...");
            let output = Command::new(bin).args(&args.args).output()?;
            check_output(&output).context("run emulator")?;
            return Ok(true);
        }
    }
    Ok(false)
}

fn run_embedded(vfs: &Path, args: &EmulatorArgs) -> Result<()> {
    // TODO(@orsinium): always use the global vfs.
    let bin_path = vfs.join("firefly-emulator");
    if !bin_path.exists() {
        println!("⏳️ downloading emulator...");
        download_emulator(&bin_path).context("download emulator")?;
    }
    println!("⌛ running...");
    let output = Command::new(bin_path).args(&args.args).output()?;
    check_output(&output).context("run emulator")?;
    Ok(())
}

fn download_emulator(bin_path: &Path) -> Result<()> {
    // Send HTTP request.
    let url = get_release_url().context("get latest release URL")?;
    let resp = ureq::get(&url).call().context("send HTTP request")?;
    let body = resp.into_body().into_reader();

    // Extract archive.
    if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
        let tar = GzDecoder::new(body);
        let mut archive = Archive::new(tar);
        let vfs = bin_path.parent().unwrap();
        archive.unpack(vfs).context("extract binary")?;
    } else {
        bail!("unsupported archive format")
    }

    // chmod.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file = File::open(bin_path).context("open file")?;
        let mut perm = file.metadata().context("read file meta")?.permissions();
        perm.set_mode(0o700);
        std::fs::set_permissions(bin_path, perm).context("set permissions")?;
    }

    Ok(())
}

fn get_release_url() -> Result<String> {
    let version = get_latest_version()?;
    let suffix = get_suffix();
    let repo = "https://github.com/firefly-zero/firefly-emulator-bin";
    Ok(format!(
        "{repo}/releases/latest/download/firefly-emulator-v{version}-{suffix}"
    ))
}

fn get_latest_version() -> Result<String> {
    let url = "https://github.com/firefly-zero/firefly-emulator-bin/releases/latest";
    let req = ureq::get(url);
    let req = req.config().max_redirects(0).build();
    let resp = req.call()?;
    if resp.status() != 302 {
        bail!("unexpected status code: {}", resp.status());
    }
    let Some(loc) = resp.headers().get("Location") else {
        bail!("no redirect Location found in response");
    };
    let loc = loc.to_str()?;
    let version = loc.split('/').next_back().unwrap();
    Ok(version.to_owned())
}

const fn get_suffix() -> &'static str {
    #[cfg(target_os = "windows")]
    return "x86_64-pc-windows-msvc.tgz";
    #[cfg(target_os = "macos")]
    return "aarch64-apple-darwin.tgz";
    #[cfg(target_os = "linux")]
    return "x86_64-unknown-linux-gnu.tgz";
}

fn binary_exists(bin: &str) -> bool {
    let output = Command::new(bin).arg("--help").output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success()
}
