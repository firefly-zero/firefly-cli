use crate::args::ImportArgs;
use crate::vfs::{get_vfs_path, init_vfs};
use anyhow::{Context, Result};
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
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("find file in archive")?;
        let out_path = out_dir.join(file.name());
        let mut outfile = File::create(&out_path).context("create output file")?;
        std::io::copy(&mut file, &mut outfile).context("copy file from archive")?;
    }
    println!("✅ installed");
    Ok(())
}

fn get_out_path(archive: &mut ZipArchive<File>) -> Result<PathBuf> {
    let mut meta_raw = Vec::new();
    let mut meta_file = archive.by_name("meta").context("open meta")?;
    meta_file.read_to_end(&mut meta_raw).context("read meta")?;
    let meta = firefly_meta::Meta::decode(&meta_raw).context("parse meta")?;
    let vfs_path = get_vfs_path();
    let rom_path = vfs_path.join("roms").join(meta.author_id).join(meta.app_id);
    Ok(rom_path)
}
