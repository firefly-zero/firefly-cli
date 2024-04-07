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

    #[serde(skip)]
    pub root: PathBuf,
}

impl Config {
    pub fn rom_path(&self) -> PathBuf {
        self.root
            .join("roms")
            .join(&self.author_id)
            .join(&self.app_id)
    }
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
