#![warn(clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::option_if_let_else)]
// #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
mod args;
mod build;
mod config;
mod export;
mod images;
mod import;
mod langs;
mod vfs;
mod wasm;
use crate::args::{Cli, Commands};
use crate::build::cmd_build;
use crate::export::cmd_export;
use crate::import::cmd_import;
use crate::vfs::cmd_vfs;
use clap::Parser;
use colored::Colorize;
use std::fmt::Display;

fn main() {
    let cli = Cli::parse();
    let res: anyhow::Result<()> = match &cli.command {
        Commands::Build(args) => cmd_build(args),
        Commands::Export(args) => cmd_export(args),
        Commands::Import(args) => cmd_import(args),
        Commands::Vfs => cmd_vfs(),
    };
    if let Err(err) = res {
        eprintln!("{} {}", "💥 Error:".red(), Error(err));
        std::process::exit(1);
    }
}

/// A wrapper for [`anyhow::Error`] that prints it as Go errors.
///
/// So, instead of:
///
/// ```text
/// 💥 Error: read config file
///
/// Caused by:
///     No such file or directory (os error 2)
/// ```
///
/// It will print:
///
/// ```text
/// 💥 Error: read config file: No such file or directory (os error 2).
/// ```
struct Error(anyhow::Error);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = &self.0;
        write!(f, "{error}")?;
        if let Some(cause) = error.source() {
            for error in anyhow::Chain::new(cause) {
                write!(f, ": {error}")?;
            }
        }
        write!(f, ".")?;
        Ok(())
    }
}
