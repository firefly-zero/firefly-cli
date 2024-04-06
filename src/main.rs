#![feature(iter_array_chunks)]

mod args;
mod build;
mod error;
mod images;
use crate::args::*;
use crate::build::cmd_build;
use crate::error::CLIError;
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    let res: Result<(), CLIError> = match &cli.command {
        Commands::Build(args) => cmd_build(args),
    };
    match res {
        Ok(_) => std::process::exit(0),
        Err(err) => err.exit(),
    }
}
