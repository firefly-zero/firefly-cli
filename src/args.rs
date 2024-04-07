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
}

#[derive(Debug, Parser)]
pub struct BuildArgs {
    /// Path to the package directory.
    #[arg(default_value = ".")]
    pub root: PathBuf,
}
