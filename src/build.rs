use crate::args::BuildArgs;
use crate::config::{Config, FileConfig};
use crate::images::convert_image;
use crate::langs::build_bin;
use anyhow::{bail, Context};

pub(crate) fn cmd_build(args: &BuildArgs) -> anyhow::Result<()> {
    let config = read_config(args)?;
    std::fs::create_dir_all(&config.rom_path).context("create rom directory")?;
    build_bin(&config).context("build binary")?;
    if let Some(files) = &config.files {
        for (name, file_config) in files.iter() {
            convert_file(name, &config, file_config).context("convert file")?;
        }
    }
    Ok(())
}

fn read_config(args: &BuildArgs) -> anyhow::Result<Config> {
    let config_path = args.root.join("firefly.toml");
    let raw_config = std::fs::read_to_string(config_path).context("read config file")?;
    let mut config: Config = toml::from_str(raw_config.as_str()).context("parse config")?;
    config.root_path = args.root.clone();
    config.roms_path = match &args.roms {
        Some(roms_path) => roms_path.clone(),
        None => config.root_path.join("roms"),
    };
    config.rom_path = config
        .roms_path
        .join(&config.author_id)
        .join(&config.app_id)
        .clone();
    Ok(config)
}

fn convert_file(name: &str, config: &Config, file_config: &FileConfig) -> anyhow::Result<()> {
    let output_path = config.rom_path.join(name);
    let Some(extension) = file_config.path.extension() else {
        let file_name = file_config.path.to_str().unwrap().to_string();
        bail!("cannot detect extension for {file_name}");
    };
    let extension = extension.to_str().unwrap();
    match extension {
        "png" => convert_image(&file_config.path, &output_path),
        _ => bail!("unknown file extension: {extension}"),
    }
}
