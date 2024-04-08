#![feature(iter_array_chunks)]

mod args;
mod build;
mod config;
mod images;
mod langs;
use crate::args::*;
use crate::build::cmd_build;
use clap::Parser;
use colored::Colorize;

fn main() {
    let cli = Cli::parse();
    let res: anyhow::Result<()> = match &cli.command {
        Commands::Build(args) => cmd_build(args),
    };
    match res {
        Ok(_) => std::process::exit(0),
        Err(err) => {
            eprintln!("{} {}", "💥 Error:".red(), err);
            std::process::exit(1);
        }
    }
}
