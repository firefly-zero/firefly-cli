use crate::args::ImportArgs;
use crate::vfs::{get_vfs_path, init_vfs};
use anyhow::{bail, Context, Result};
use firefly_meta::Meta;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::PathBuf;
use zip::ZipArchive;

pub(crate) fn cmd_import(args: &ImportArgs) -> Result<()> {
    let file = File::open(&args.path).context("open archive file")?;
    let mut archive = ZipArchive::new(file).context("open archive")?;
    let out_dir = get_out_path(&mut archive).context("get ROM path")?;
    init_vfs().context("init VFS")?;
    create_dir_all(&out_dir).context("create ROM dir")?;
    archive.extract(out_dir).context("extract archive")?;
    println!("✅ installed");
    Ok(())
}

fn get_out_path(archive: &mut ZipArchive<File>) -> Result<PathBuf> {
    let mut meta_raw = Vec::new();
    let mut meta_file = archive.by_name("meta").context("open meta")?;
    meta_file.read_to_end(&mut meta_raw).context("read meta")?;
    if meta_raw.is_empty() {
        bail!("meta is empty");
    }
    let meta = Meta::decode(&meta_raw).context("parse meta")?;
    let vfs_path = get_vfs_path();
    let rom_path = vfs_path.join("roms").join(meta.author_id).join(meta.app_id);
    Ok(rom_path)
}
