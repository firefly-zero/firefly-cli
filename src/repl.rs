use crate::args::{Cli, ReplArgs};
use crate::cli::{run_command, Error};
use anyhow::{Context, Result};
use clap::Parser;
use crossterm::style::Stylize;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;
use std::path::Path;

pub fn cmd_repl(vfs: &Path, _args: &ReplArgs) -> Result<()> {
    let mut rl: Editor<(), FileHistory> = Editor::new().unwrap();
    if rl.load_history(".history.txt").is_err() {
        println!("{}", "No previous history.".yellow());
    }
    let mut was_ok = true;
    loop {
        let prompt = if was_ok { ">>> ".green() } else { ">>> ".red() };
        let readline = rl.readline(&prompt.to_string());
        match readline {
            Ok(input) => {
                let mut args: Vec<_> = input.split_ascii_whitespace().collect();
                args.insert(0, "firefly_cli");
                let cli = match Cli::try_parse_from(args) {
                    Ok(cli) => cli,
                    Err(err) => {
                        eprintln!("{} {}", "💥 Error:".red(), Error(err.into()));
                        was_ok = false;
                        continue;
                    }
                };
                let res = run_command(vfs.to_owned(), &cli.command);
                if let Err(err) = res {
                    eprintln!("{} {}", "💥 Error:".red(), Error(err));
                    was_ok = false;
                    continue;
                }
                was_ok = true;
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "CTRL-C".yellow());
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "CTRL-D".yellow());
                break;
            }
            Err(err) => {
                println!("{}", err.to_string().red());
                break;
            }
        }
    }
    rl.save_history(".history.txt").context("save history")?;
    Ok(())
}
