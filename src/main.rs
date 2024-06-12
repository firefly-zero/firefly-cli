#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::option_if_let_else)]

mod args;
mod build;
mod config;
mod export;
mod file_names;
mod images;
mod import;
mod keys;
mod langs;
mod vfs;
mod wasm;

use crate::args::{Cli, Commands};
use crate::build::cmd_build;
use crate::export::cmd_export;
use crate::import::cmd_import;
use crate::vfs::cmd_vfs;
use args::KeyCommands;
use clap::Parser;
use colored::Colorize;
use keys::{cmd_key_add, cmd_key_new, cmd_key_priv, cmd_key_pub, cmd_key_rm};
use std::fmt::Display;

fn main() {
    let cli = Cli::parse();
    let res: anyhow::Result<()> = match &cli.command {
        Commands::Build(args) => cmd_build(args),
        Commands::Export(args) => cmd_export(args),
        Commands::Import(args) => cmd_import(args),
        Commands::Key(KeyCommands::New(args)) => cmd_key_new(args),
        Commands::Key(KeyCommands::Add(args)) => cmd_key_add(args),
        Commands::Key(KeyCommands::Pub(args)) => cmd_key_pub(args),
        Commands::Key(KeyCommands::Priv(args)) => cmd_key_priv(args),
        Commands::Key(KeyCommands::Rm(args)) => cmd_key_rm(args),
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
