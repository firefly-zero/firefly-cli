use clap::{Parser, Subcommand};

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
    #[arg(short, long, default_value = ".")]
    pub input: String,

    /// Path to the file where the cartridge will be saved.
    #[arg(short, long, default_value = None)]
    pub output: Option<String>,

    /// The programming language used in the app.
    #[arg(short, long, default_value = None)]
    pub lang: Option<String>,
}
