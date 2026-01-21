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
    #[cfg(target_os = "windows")]
    const SUFFIX: &str = "x86_64-pc-windows-msvc.tgz";
    #[cfg(target_os = "macos")]
    const SUFFIX: &str = "aarch64-apple-darwin.tgz";
    #[cfg(target_os = "linux")]
    const SUFFIX: &str = "x86_64-unknown-linux-gnu.tgz";

    let version = get_latest_version()?;
    let repo = "https://github.com/firefly-zero/firefly-emulator";
    Ok(format!(
        "{repo}/releases/latest/download/firefly-emulator-v{version}-{SUFFIX}"
    ))
}

fn get_latest_version() -> Result<String> {
    let url = "https://github.com/firefly-zero/firefly-emulator/releases/latest";
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
