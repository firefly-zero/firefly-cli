use crate::args::ImportArgs;
use crate::vfs::{get_vfs_path, init_vfs};
use anyhow::{bail, Context, Result};
use firefly_meta::Meta;
use std::fs::{self, create_dir_all, File};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub fn cmd_import(args: &ImportArgs) -> Result<()> {
    let file = File::open(&args.path).context("open archive file")?;
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

fn read_meta_raw(archive: &mut ZipArchive<File>) -> Result<Vec<u8>> {
    let mut meta_raw = Vec::new();
    let mut meta_file = archive.by_name("meta").context("open meta")?;
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
