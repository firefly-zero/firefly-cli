#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::enum_glob_use)]

mod args;
mod build;
mod catalog;
mod cheat;
mod config;
mod crypto;
mod export;
mod file_names;
mod images;
mod import;
mod keys;
mod langs;
mod monitor;
mod vfs;
mod wasm;

mod net;
#[cfg(test)]
mod test_helpers;

use crate::args::{Cli, Commands, KeyCommands};
use crate::build::cmd_build;
use crate::catalog::{cmd_catalog_list, cmd_catalog_show};
use crate::cheat::cmd_cheat;
use crate::export::cmd_export;
use crate::import::cmd_import;
use crate::keys::{cmd_key_add, cmd_key_new, cmd_key_priv, cmd_key_pub, cmd_key_rm};
use crate::monitor::cmd_monitor;
use crate::vfs::{cmd_vfs, get_vfs_path};
use args::CatalogCommands;
use clap::Parser;
use crossterm::style::Stylize;
use std::fmt::Display;

fn main() {
    let cli = Cli::parse();
    let vfs = get_vfs_path();
    let res: anyhow::Result<()> = match &cli.command {
        Commands::Build(args) => cmd_build(vfs, args),
        Commands::Export(args) => cmd_export(&vfs, args),
        Commands::Import(args) => cmd_import(&vfs, args),
        Commands::Cheat(args) => cmd_cheat(args),
        Commands::Monitor(args) => cmd_monitor(&vfs, args),
        Commands::Key(KeyCommands::New(args)) => cmd_key_new(&vfs, args),
        Commands::Key(KeyCommands::Add(args)) => cmd_key_add(&vfs, args),
        Commands::Key(KeyCommands::Pub(args)) => cmd_key_pub(&vfs, args),
        Commands::Key(KeyCommands::Priv(args)) => cmd_key_priv(&vfs, args),
        Commands::Key(KeyCommands::Rm(args)) => cmd_key_rm(&vfs, args),
        Commands::Catalog(CatalogCommands::List(args)) => cmd_catalog_list(args),
        Commands::Catalog(CatalogCommands::Show(args)) => cmd_catalog_show(args),
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
