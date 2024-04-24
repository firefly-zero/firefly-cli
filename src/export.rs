use crate::args::ExportArgs;
use crate::config::Config;
use crate::vfs::get_vfs_path;
use anyhow::Context;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub(crate) fn cmd_export(args: &ExportArgs) -> anyhow::Result<()> {
    let (author_id, app_id) = get_id(args)?;
    let vfs_path = get_vfs_path();
    let rom_path = vfs_path.join("roms").join(&author_id).join(&app_id);
    let out_path: PathBuf = match &args.output {
        Some(out_path) => out_path.clone(),
        None => format!("{author_id}.{app_id}.zip").into(),
    };
    archive(&rom_path, &out_path).context("create archive")?;
    let out_path = out_path.as_os_str();
    if let Some(out_path) = out_path.to_str() {
        println!("{out_path}");
    }
    Ok(())
}

fn get_id(args: &ExportArgs) -> anyhow::Result<(String, String)> {
    match (&args.author, &args.app) {
        (Some(author), Some(app)) => Ok((author.to_string(), app.to_string())),
        _ => {
            let config = Config::load(&args.root).context("read project config")?;
            Ok((config.author_id, config.app_id))
        }
    }
}

fn archive(in_path: &Path, out_path: &Path) -> anyhow::Result<()> {
    // Should go first so that we don't create empty archive
    // if ROM doesn't exist.
    let entries = fs::read_dir(in_path).context("read ROM dir")?;

    let out_file = fs::File::create(out_path).context("create archive file")?;
    let mut zip = zip::ZipWriter::new(out_file);
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Zstd)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in entries {
        let entry = entry.context("get dir entry")?;
        let file_path = entry.file_name();
        let file_path = file_path.to_str().unwrap();
        let file_path = file_path.to_string();
        zip.start_file(file_path, options)
            .context("create file in archive")?;
        let path = entry.path();
        let mut f = fs::File::open(path).context("open file in ROM")?;
        f.read_to_end(&mut buffer).context("read file")?;
        zip.write_all(&buffer).context("write file into archive")?;
    }
    Ok(())
}
