#![forbid(unsafe_code)]
#![deny(
    rust_2018_idioms,
    redundant_lifetimes,
    redundant_semicolons,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::allow_attributes
)]
#![allow(
    clippy::enum_glob_use,
    clippy::wildcard_imports,
    clippy::case_sensitive_file_extension_comparisons
)]
#![expect(clippy::option_if_let_else)]

mod args;
mod audio;
mod cli;
mod commands;
mod config;
mod crypto;
mod file_names;
mod fs;
mod images;
mod langs;
mod net;
mod repl_helper;
mod vfs;
mod wasm;

mod palettes;
#[cfg(test)]
mod test_helpers;

use crate::args::Cli;
use crate::cli::{Error, run_command};
use crate::vfs::get_vfs_path;
use clap::Parser;
use crossterm::style::Stylize;

fn main() {
    let cli = Cli::parse();
    let vfs = match cli.vfs {
        Some(vfs) => vfs,
        None => get_vfs_path(),
    };
    let res = run_command(vfs, &cli.command);
    if let Err(err) = res {
        eprintln!("{} {}", "💥 Error:".red(), Error(err));
        std::process::exit(1);
    }
}
