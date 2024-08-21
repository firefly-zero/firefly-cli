#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::enum_glob_use)]

mod args;
mod build;
mod catalog;
mod cheat;
mod cli;
mod config;
mod crypto;
mod export;
mod file_names;
mod images;
mod import;
mod inspect;
mod keys;
mod langs;
mod monitor;
mod net;
mod repl;
mod repl_helper;
mod vfs;
mod wasm;

#[cfg(test)]
mod test_helpers;

use crate::args::Cli;
use crate::cli::{run_command, Error};
use crate::vfs::get_vfs_path;
use clap::Parser;
use crossterm::style::Stylize;

fn main() {
    let cli = Cli::parse();
    let vfs = get_vfs_path();
    let res = run_command(vfs, &cli.command);
    if let Err(err) = res {
        eprintln!("{} {}", "💥 Error:".red(), Error(err));
        std::process::exit(1);
    }
}
