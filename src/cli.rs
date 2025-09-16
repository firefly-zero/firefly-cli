use crate::args::*;
use crate::commands::*;
use std::fmt::Display;
use std::path::PathBuf;

pub fn run_command(vfs: PathBuf, command: &Commands) -> anyhow::Result<()> {
    match command {
        Commands::Build(args) => cmd_build(vfs, args),
        Commands::Export(args) => cmd_export(&vfs, args),
        Commands::Import(args) => cmd_import(&vfs, args),
        Commands::New(args) => cmd_new(args),
        Commands::Emulator(args) => cmd_emulator(args),
        Commands::Badges(args) => cmd_badges(&vfs, args),
        Commands::Boards(args) => cmd_boards(&vfs, args),
        Commands::Cheat(args) => cmd_cheat(args),
        Commands::Monitor(args) => cmd_monitor(&vfs, args),
        Commands::Logs(args) => cmd_logs(args),
        Commands::Inspect(args) => cmd_inspect(&vfs, args),
        Commands::Repl(args) => cmd_repl(&vfs, args),
        Commands::Shots(ShotsCommands::Download(args)) => cmd_shots_download(&vfs, args),
        Commands::Key(KeyCommands::New(args)) => cmd_key_new(&vfs, args),
        Commands::Key(KeyCommands::Add(args)) => cmd_key_add(&vfs, args),
        Commands::Key(KeyCommands::Pub(args)) => cmd_key_pub(&vfs, args),
        Commands::Key(KeyCommands::Priv(args)) => cmd_key_priv(&vfs, args),
        Commands::Key(KeyCommands::Rm(args)) => cmd_key_rm(&vfs, args),
        Commands::Catalog(CatalogCommands::List(args)) => cmd_catalog_list(args),
        Commands::Catalog(CatalogCommands::Show(args)) => cmd_catalog_show(args),
        Commands::Name(NameCommands::Get) => cmd_name_get(&vfs),
        Commands::Name(NameCommands::Set(args)) => cmd_name_set(&vfs, args),
        Commands::Name(NameCommands::Generate) => cmd_name_generate(&vfs),
        Commands::Runtime(root_args) => match &root_args.command {
            RuntimeCommands::Restart => cmd_restart(root_args),
            RuntimeCommands::Exit => cmd_exit(root_args),
        },
        Commands::Vfs => cmd_vfs(),
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
pub struct Error(pub anyhow::Error);

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
