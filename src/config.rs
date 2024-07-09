use anyhow::Context;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub app_id: String,
    pub author_id: String,
    pub app_name: String,
    pub author_name: String,

    /// The app version. Compared between devices when starting multiplayer.
    #[serde(default)]
    pub version: Option<u32>,

    /// The programming language used for the app.
    pub lang: Option<Lang>,

    /// Additional CLI args to pass into the build subcommand.
    pub compile_args: Option<Vec<String>>,

    /// The app should be launched first when the device starts.
    #[serde(default)]
    pub launcher: bool,

    /// The app requires privileged access.
    #[serde(default)]
    pub sudo: bool,

    /// Mapping of local files to be included into the ROM.
    pub files: Option<HashMap<String, FileConfig>>,

    /// Path to the project root.
    #[serde(skip)]
    pub root_path: PathBuf,

    /// Path to the root of the virtual filesystem.
    #[serde(skip)]
    pub vfs_path: PathBuf,

    /// Path to the root of the current app.
    #[serde(skip)]
    pub rom_path: PathBuf,
}

impl Config {
    pub fn load(vfs: PathBuf, root: &Path) -> anyhow::Result<Self> {
        let config_path = root.join("firefly.toml");
        let raw_config = fs::read_to_string(config_path).context("read config file")?;
        let mut config: Self = toml::from_str(raw_config.as_str()).context("parse config")?;
        config.root_path = match std::env::current_dir() {
            // Make the path absolute if possible
            Ok(current_dir) => current_dir.join(root),
            Err(_) => PathBuf::from(root),
        };
        config.vfs_path = vfs;
        config.rom_path = config
            .vfs_path
            .join("roms")
            .join(&config.author_id)
            .join(&config.app_id);
        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    /// Path to the file relative to the project root.
    pub path: PathBuf,

    /// URL to download the file from if missed.
    pub url: Option<String>,

    /// The file hash to validate when downloading the file.
    pub sha256: Option<String>,

    /// If the file should be copied as-is, without any processing.
    #[serde(default)]
    pub copy: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Go,
    Rust,
    Zig,
    TS,
    C,
    Cpp,
    Python,
}
