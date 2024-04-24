use crate::args::BuildArgs;
use crate::config::{Config, FileConfig};
use crate::images::convert_image;
use crate::langs::build_bin;
use crate::vfs::init_vfs;
use anyhow::{bail, Context};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(crate) fn cmd_build(args: &BuildArgs) -> anyhow::Result<()> {
    init_vfs().context("init vfs")?;
    let config = Config::load(&args.root).context("load project config")?;
    let old_sizes = collect_sizes(&config.rom_path);
    write_meta(&config).context("write metadata file")?;
    build_bin(&config).context("build binary")?;
    if let Some(files) = &config.files {
        for (name, file_config) in files.iter() {
            convert_file(name, &config, file_config).context("convert file")?;
        }
    }
    write_installed(&config)?;
    let new_sizes = collect_sizes(&config.rom_path);
    print_sizes(old_sizes, new_sizes);
    Ok(())
}

fn write_meta(config: &Config) -> anyhow::Result<()> {
    use firefly_meta::*;
    if let Err(err) = validate_id(&config.app_id) {
        bail!("validate app_id: {err}");
    }
    if let Err(err) = validate_id(&config.author_id) {
        bail!("validate author_id: {err}");
    }
    if let Err(err) = validate_name(&config.app_name) {
        bail!("validate app_name: {err}");
    }
    if let Err(err) = validate_name(&config.author_name) {
        bail!("validate author_name: {err}");
    }
    let meta = Meta {
        app_id:      &config.app_id,
        app_name:    &config.app_name,
        author_id:   &config.author_id,
        author_name: &config.author_name,
        launcher:    config.launcher,
        sudo:        config.sudo,
    };
    let mut buf = vec![0; meta.size()];
    let encoded = meta.encode(&mut buf).context("serialize")?;
    fs::create_dir_all(&config.rom_path)?;
    let output_path = config.rom_path.join("meta");
    fs::write(output_path, encoded).context("write file")?;
    Ok(())
}

/// Write the latest installed app name into internal DB.
fn write_installed(config: &Config) -> anyhow::Result<()> {
    let meta = firefly_meta::ShortMeta {
        app_id:    &config.app_id,
        author_id: &config.author_id,
    };
    let mut buf = vec![0; meta.size()];
    let encoded = meta.encode(&mut buf).context("serialize")?;
    let output_path = config.vfs_path.join("sys").join("new-app");
    fs::write(output_path, &encoded).context("write new-app file")?;
    if config.launcher {
        let output_path = config.vfs_path.join("sys").join("launcher");
        fs::write(output_path, &encoded).context("write launcher file")?;
    }
    Ok(())
}

fn convert_file(name: &str, config: &Config, file_config: &FileConfig) -> anyhow::Result<()> {
    let output_path = config.rom_path.join(name);
    // The input path is defined in the config
    // and should be resolved relative to the project root.
    let input_path = &config.root_path.join(&file_config.path);
    let Some(extension) = input_path.extension() else {
        let file_name = input_path.to_str().unwrap().to_string();
        bail!("cannot detect extension for {file_name}");
    };
    let extension = match extension.to_str() {
        Some(extension) => extension,
        None => bail!("cannot convert file extension to string"),
    };
    match extension {
        "png" => {
            convert_image(input_path, &output_path)?;
        }
        // firefly formats for fonts and images
        "fff" | "ffi" => {
            fs::copy(input_path, &output_path)?;
        }
        _ => bail!("unknown file extension: {extension}"),
    }
    Ok(())
}

fn collect_sizes(root: &Path) -> HashMap<OsString, u64> {
    let mut sizes = HashMap::new();
    let Ok(entries) = fs::read_dir(root) else {
        return sizes;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(meta) = entry.metadata() else { continue };
        sizes.insert(entry.file_name(), meta.size());
    }
    sizes
}

fn print_sizes(old_sizes: HashMap<OsString, u64>, new_sizes: HashMap<OsString, u64>) {
    let mut pairs: Vec<_> = new_sizes.iter().collect();
    pairs.sort();
    for (name, new_size) in pairs {
        let old_size = old_sizes.get(name).unwrap_or(&0);
        let Some(name) = name.to_str() else {
            continue;
        };
        // If the size changed, show the diff
        let suffix = if old_size != new_size {
            let diff = *new_size as i64 - *old_size as i64;
            format!(" ({diff:+})")
        } else {
            "".to_string()
        };
        println!("{name:16} {new_size:>7}{suffix}")
    }
}
