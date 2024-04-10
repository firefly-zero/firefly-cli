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
    /// Build cartridge.
    Build(BuildArgs),

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
