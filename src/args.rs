#![allow(clippy::module_name_repetitions)]

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the project and install it locally (into VFS).
    Build(BuildArgs),

    /// Export an installed app as a zip archive.
    Export(ExportArgs),

    /// Install locally an app from a zip archive.
    Import(ImportArgs),

    /// Show the full path to the virtual filesystem.
    Vfs,
}

#[derive(Debug, Parser)]
pub struct BuildArgs {
    /// Path to the project root.
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Path to the directory where to store roms
    #[arg(short, long, default_value = None)]
    pub roms: Option<PathBuf>,

    /// Path to the firefly config.
    #[arg(short, long, default_value = None)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ExportArgs {
    /// Path to the project root.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Author ID.
    #[arg(long, default_value = None)]
    pub author: Option<String>,

    /// App ID.
    #[arg(long, default_value = None)]
    pub app: Option<String>,

    /// Path to the archive.
    #[arg(short, long, default_value = None)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ImportArgs {
    /// The ROM to install.
    ///
    /// The ROM can be one of:
    ///
    /// 1. Local path to a zip file (for example, `sys.launcher.zip`)
    ///
    /// 2. URL of a zip file (for example, `https://example.com/sys.launcher.zip`)
    ///
    /// 3. App ID in the catalog (for example, `sys.launcher`).
    ///
    /// 4. The word "launcher" to install the latest version of the default launcher.
    #[arg()]
    pub path: String,
}
