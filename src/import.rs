use crate::args::ImportArgs;
use crate::file_names::META;
use crate::vfs::{get_vfs_path, init_vfs};
use anyhow::{bail, Context, Result};
use firefly_meta::Meta;
use serde::Deserialize;
use std::env::temp_dir;
use std::fs::{self, create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// API response from the firefly catalog.
///
/// Example: <https://catalog.fireflyzero.com/sys.launcher.json>
#[derive(Deserialize)]
struct CatalogApp {
    download: String,
}

pub fn cmd_import(args: &ImportArgs) -> Result<()> {
    let path = fetch_archive(&args.path).context("download ROM archive")?;
    let file = File::open(path).context("open archive file")?;
    let mut archive = ZipArchive::new(file).context("open archive")?;

    let meta_raw = read_meta_raw(&mut archive)?;
    let meta = Meta::decode(&meta_raw).context("parse meta")?;
    let vfs_path = get_vfs_path();
    let rom_path = vfs_path.join("roms").join(meta.author_id).join(meta.app_id);

    init_vfs().context("init VFS")?;
    create_dir_all(&rom_path).context("create ROM dir")?;
    archive.extract(&rom_path).context("extract archive")?;
    if let Some(rom_path) = rom_path.to_str() {
        println!("✅ installed: {rom_path}");
    }
    write_installed(&meta, &vfs_path)?;
    Ok(())
}

fn fetch_archive(path: &str) -> Result<PathBuf> {
    let mut path = path.to_string();
    if path == "launcher" {
        path = "https://github.com/firefly-zero/firefly-launcher/releases/latest/download/sys.launcher.zip".to_string();
    }

    // App ID is given. Fetch download URL from the catalog.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if !path.ends_with(".zip") {
        let url = format!("https://catalog.fireflyzero.com/{path}.json");
        let resp = ureq::get(&url).call().context("send HTTP request")?;
        if resp.status() == 200 && resp.header("Content-Type") == Some("application/json") {
            let app: CatalogApp =
                serde_json::from_reader(&mut resp.into_reader()).context("parse JSON")?;
            path = app.download;
        }
    }

    // Local path is given. Just use it.
    if !path.starts_with("https://") {
        return Ok(path.into());
    }

    // URL is given. Download into a temporary file.
    println!("⏳️ downloading the file...");
    let resp = ureq::get(&path).call().context("send HTTP request")?;
    let out_path = temp_dir().join("rom.zip");
    let mut file = File::create(&out_path)?;
    std::io::copy(&mut resp.into_reader(), &mut file).context("write response into a file")?;
    println!("⌛ installing...");
    Ok(out_path)
}

fn read_meta_raw(archive: &mut ZipArchive<File>) -> Result<Vec<u8>> {
    let mut meta_raw = Vec::new();
    let mut meta_file = archive.by_name(META).context("open meta")?;
    meta_file.read_to_end(&mut meta_raw).context("read meta")?;
    if meta_raw.is_empty() {
        bail!("meta is empty");
    }
    Ok(meta_raw)
}

/// Write the latest installed app name into internal DB.
fn write_installed(meta: &Meta, vfs_path: &Path) -> anyhow::Result<()> {
    let short_meta = firefly_meta::ShortMeta {
        app_id:    meta.app_id,
        author_id: meta.author_id,
    };
    let mut buf = vec![0; short_meta.size()];
    let encoded = short_meta.encode(&mut buf).context("serialize")?;
    let output_path = vfs_path.join("sys").join("new-app");
    fs::write(output_path, &encoded).context("write new-app file")?;
    if meta.launcher {
        let output_path = vfs_path.join("sys").join("launcher");
        fs::write(output_path, &encoded).context("write launcher file")?;
    }
    Ok(())
}
