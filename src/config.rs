use crate::vfs::get_vfs_path;
use anyhow::Context;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    pub app_id:      String,
    pub author_id:   String,
    pub app_name:    String,
    pub author_name: String,

    /// The programming language used for the app.
    pub lang: Option<Lang>,

    /// Additional CLI args to pass into the build subcommand.
    pub compile_args: Option<Vec<String>>,

    /// The app should be launched first when the device starts.
    #[serde(default)]
    pub launcher: bool,

    /// The app requires privileged access.
    #[serde(default)]
    pub sudo:  bool,
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
    pub(crate) fn load(root: &Path) -> anyhow::Result<Self> {
        let config_path = root.join("firefly.toml");
        let raw_config = fs::read_to_string(config_path).context("read config file")?;
        let mut config: Config = toml::from_str(raw_config.as_str()).context("parse config")?;
        config.root_path = match std::env::current_dir() {
            // Make the path absolute if possible
            Ok(current_dir) => current_dir.join(root),
            Err(_) => PathBuf::from(root),
        };
        config.vfs_path = get_vfs_path();
        config.rom_path = config
            .vfs_path
            .join("roms")
            .join(&config.author_id)
            .join(&config.app_id);
        Ok(config)
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct FileConfig {
    pub path:   PathBuf,
    pub url:    Option<String>,
    pub sha256: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Lang {
    Go,
    Rust,
    Zig,
    TS,
}
