use crate::args::BuildArgs;
use crate::config::{Config, FileConfig};
use crate::images::convert_image;
use crate::langs::build_bin;
use crate::vfs::init_vfs;
use anyhow::{bail, Context};
use std::fs;

pub(crate) fn cmd_build(args: &BuildArgs) -> anyhow::Result<()> {
    init_vfs().context("init vfs")?;
    let config = Config::load(&args.root).context("load project config")?;
    write_meta(&config).context("write metadata file")?;
    build_bin(&config).context("build binary")?;
    if let Some(files) = &config.files {
        for (name, file_config) in files.iter() {
            convert_file(name, &config, file_config).context("convert file")?;
        }
    }
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
    };
    let mut buf = vec![0; meta.size()];
    let encoded = meta.encode(&mut buf).context("serialize")?;
    let output_path = config.rom_path.join("meta");
    fs::write(output_path, encoded).context("write file")?;
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
