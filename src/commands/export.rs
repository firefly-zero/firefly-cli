use crate::args::ExportArgs;
use crate::config::Config;
use anyhow::{bail, Context, Result};
use std::fs::{read_dir, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn cmd_export(vfs: &Path, args: &ExportArgs) -> Result<()> {
    let (author_id, app_id) = get_id(vfs.to_path_buf(), args)?;
    let rom_path = vfs.join("roms").join(&author_id).join(&app_id);
    let out_path: PathBuf = match &args.output {
        Some(out_path) => out_path.clone(),
        None => format!("{author_id}.{app_id}.zip").into(),
    };
    archive(&rom_path, &out_path).context("create archive")?;
    let out_path = out_path.as_os_str();
    if let Some(out_path) = out_path.to_str() {
        println!("✅ exported: {out_path}");
    }
    Ok(())
}

fn get_id(vfs: PathBuf, args: &ExportArgs) -> Result<(String, String)> {
    let res = if let Some(id) = &args.id {
        let Some((author_id, app_id)) = id.split_once('.') else {
            bail!("invalid app id: dot not found");
        };
        (author_id.to_string(), app_id.to_string())
    } else {
        let config = Config::load(vfs, &args.root).context("read project config")?;
        (config.author_id, config.app_id)
    };
    Ok(res)
}

fn archive(in_path: &Path, out_path: &Path) -> Result<()> {
    // Should go first so that we don't create empty archive
    // if ROM doesn't exist.
    let entries = read_dir(in_path).context("read ROM dir")?;

    let out_file = File::create(out_path).context("create archive file")?;
    let mut zip = ZipWriter::new(out_file);
    let options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Zstd)
        .unix_permissions(0o755);

    for entry in entries {
        let entry = entry.context("get dir entry")?;
        let file_path = entry.file_name();
        let file_path = file_path.to_str().unwrap();
        let file_path = file_path.to_string();
        zip.start_file(file_path, options)
            .context("create file in archive")?;
        let path = entry.path();
        let mut file = File::open(path).context("open file in ROM")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).context("read file")?;
        zip.write_all(&buffer).context("write file into archive")?;
    }
    Ok(())
}
