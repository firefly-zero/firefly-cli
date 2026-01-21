use crate::args::EmulatorArgs;
use crate::langs::check_exit_status;
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
    let exit_status = Command::new(bin_path).args(&args.args).status()?;
    check_exit_status(exit_status).context("run emulator")?;
    Ok(())
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
    #[cfg(target_os = "windows")]
    const SUFFIX: &str = "x86_64-pc-windows-msvc.tgz";
    #[cfg(target_os = "macos")]
    const SUFFIX: &str = "aarch64-apple-darwin.tgz";
    #[cfg(target_os = "linux")]
    const SUFFIX: &str = "x86_64-unknown-linux-gnu.tgz";

    let repo = "https://github.com/firefly-zero/firefly-emulator";
    format!("{repo}/releases/latest/download/firefly-emulator-{SUFFIX}")
}
