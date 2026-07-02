use crate::args::EmulatorArgs;
use crate::langs::run_cmd;
use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use tar::Archive;

pub fn cmd_emulator(vfs: &Path, args: &EmulatorArgs) -> Result<()> {
    #[cfg(target_os = "windows")]
    const BINARY_NAME: &str = "firefly-emulator.exe";
    #[cfg(not(target_os = "windows"))]
    const BINARY_NAME: &str = "firefly-emulator";

    // TODO(@orsinium): always use the global vfs.
    let bin_path = vfs.join(BINARY_NAME);
    if args.update || !bin_path.exists() {
        println!("⏳️ downloading emulator...");
        download_emulator(&bin_path).context("download emulator")?;
    }
    println!("⌛ running...");
    run_cmd(Command::new(bin_path).args(format_args(args))).context("run emulator")?;
    Ok(())
}

fn format_args(args: &EmulatorArgs) -> Vec<&str> {
    let mut res = Vec::new();
    if args.update {
        res.push("--update");
    }
    if args.fullscreen {
        res.push("--fullscreen");
    }
    if args.no_keyboard {
        res.push("--no_keyboard");
    }
    if args.mute {
        res.push("--mute");
    }

    if let Some(val) = &args.scale {
        res.push("--scale");
        res.push(val);
    }
    if let Some(val) = &args.id {
        res.push("--id");
        res.push(val);
    }
    if let Some(val) = &args.tcp_ip {
        res.push("--tcp_ip");
        res.push(val);
    }
    if let Some(val) = &args.udp_ip {
        res.push("--udp_ip");
        res.push(val);
    }
    if let Some(val) = &args.peers {
        res.push("--peers");
        res.push(val);
    }
    if let Some(val) = &args.vfs {
        res.push("--vfs");
        res.push(val);
    }
    if let Some(val) = &args.wav {
        res.push("--wav");
        res.push(val);
    }
    res
}

fn download_emulator(bin_path: &Path) -> Result<()> {
    // Send HTTP request.
    let url = get_release_url();
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

fn get_release_url() -> String {
    let (arch, abi) = cfg_select! {
        target_arch = "x86_64" => ("x86_64", ""),
        target_arch = "x86" => ("i686", ""),
        target_arch = "aarch64" => ("aarch64", ""),
        target_arch = "arm" => ("arm", "eabihf"),
        _ => compile_error!("unsupported architecture"),
    };
    let os = cfg_select! {
        target_os = "windows" => "windows-msvc",
        target_os = "macos" => "apple-darwin",
        target_os = "linux" => "unknown-linux-gnu",
        _ => compile_error!("unsupported os"),
    };

    let repo = "https://github.com/firefly-zero/firefly-emulator";
    format!("{repo}/releases/latest/download/firefly-emulator-{arch}-{os}{abi}.tgz")
}
