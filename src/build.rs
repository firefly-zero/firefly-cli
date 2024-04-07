use crate::args::BuildArgs;
use crate::config::{Config, FileConfig};
use crate::error::CLIError;
use crate::images::convert_image;
use crate::langs::build_bin;

pub(crate) fn cmd_build(args: &BuildArgs) -> Result<(), CLIError> {
    let raw_config = std::fs::read_to_string(&args.root)?;
    let config: Config = toml::from_str(raw_config.as_str())?;
    build_bin(&config)?;
    for (name, file_config) in config.files.iter() {
        convert_file(name, &config, file_config)?;
    }
    Ok(())
}

fn convert_file(name: &str, config: &Config, file_config: &FileConfig) -> Result<(), CLIError> {
    let output_path = config.rom_path().join(name);
    let Some(extension) = file_config.path.extension() else {
        let file_name = file_config.path.to_str().unwrap().to_string();
        return Err(CLIError::FileExtNotDetected(file_name));
    };
    let extension = extension.to_str().unwrap();
    match extension {
        "png" => convert_image(&file_config.path, &output_path),
        _ => Err(CLIError::UnknownFileExt(extension.to_string())),
    }
}
