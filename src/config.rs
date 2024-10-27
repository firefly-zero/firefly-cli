use anyhow::{bail, Context, Result};
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

    /// Mapping of cheat commands to their integer representation.
    pub cheats: Option<HashMap<String, i32>>,

    /// Mapping of badge IDs to badges.
    pub badges: Option<HashMap<u16, BadgeConfig>>,

    /// Mapping of board IDs to boards.
    pub boards: Option<HashMap<u16, BoardConfig>>,

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

    pub fn badges_vec(&self) -> Result<Vec<&BadgeConfig>> {
        let Some(badges_config) = &self.badges else {
            return Ok(Vec::new());
        };
        if badges_config.get(&1).is_none() || badges_config.get(&0).is_some() {
            bail!("badge IDs must start at 1")
        }
        let len = badges_config.len();
        if len > 200 {
            bail!("too many badges")
        }
        let len = u16::try_from(len).unwrap();
        let mut badges = Vec::new();
        for id in 1u16..=len {
            let Some(badge) = badges_config.get(&id) else {
                bail!("badge IDs must be consequentive but ID {id} is missed");
            };
            badges.push(badge);
        }
        Ok(badges)
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

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BadgeConfig {
    /// Human-readable achievement name.
    pub name: String,

    /// Human-readable achievement description. Typically, a hint on how to earn it.
    #[serde(default)]
    pub descr: String,

    /// The order in which achievement should be displayed, ascending.
    ///
    /// Defaults to the achievement's ID.
    ///
    /// Earned achievments bubble up.
    pub position: Option<u16>,

    /// How many steps there are to earn the badge.
    ///
    /// Defaults to 1.
    pub steps: Option<u16>,

    /// How much XP earning the achievement brings to the player.
    #[serde(default)]
    pub xp: u8,

    /// If the achievement should be hidden until earned.
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BoardConfig {
    /// Human-readable board name.
    pub name: String,

    /// The order in which the board should be displayed, ascending.
    pub position: Option<u16>,

    /// The minimum value for a score to be added to the board.
    pub min: Option<u32>,

    /// The maximum value for a score to be added to the board.
    ///
    /// Useful for filtering out obvious cheating.
    pub max: Option<u32>,

    /// If the scores should go in ascending order.
    ///
    /// If false (default), uses descending ("larger is better") order.
    /// Ascending order makes sense for time in racing games.
    #[serde(default)]
    pub asc: bool,

    /// If the score should be formatted as time.
    #[serde(default)]
    pub time: bool,

    /// Digits after decimal point.
    #[serde(default)]
    pub decimals: u8,
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
