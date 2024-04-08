use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    pub app_id:      String,
    pub author_id:   String,
    pub app_name:    String,
    pub author_name: String,
    pub lang:        Option<Lang>,
    pub files:       Option<HashMap<String, FileConfig>>,

    /// Path to the project root.
    #[serde(skip)]
    pub root_path: PathBuf,

    /// Path to the directory with ROMs for all apps.
    #[serde(skip)]
    pub roms_path: PathBuf,

    /// Path to the room of the current app.
    #[serde(skip)]
    pub rom_path: PathBuf,

    /// Path to the file with the config.
    #[serde(skip)]
    pub config_path: PathBuf,
}

#[derive(Deserialize, Debug)]
pub(crate) struct FileConfig {
    pub path: PathBuf,
    // pub url:  String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Lang {
    Go,
    Rust,
    Zig,
    TS,
}
