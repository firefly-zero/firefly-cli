use crate::args::*;
use crate::commands::*;
use std::fmt::Display;
use std::path::PathBuf;

pub fn run_command(vfs: PathBuf, command: &Commands) -> anyhow::Result<()> {
    use Commands::*;
    match command {
        Build(args) => cmd_build(vfs, args),
        Export(args) => cmd_export(&vfs, args),
        Import(args) => cmd_import(&vfs, args),
        New(args) => cmd_new(args),
        Test(args) => cmd_test(args),
        Emulator(args) => cmd_emulator(args),
        Badges(args) => cmd_badges(&vfs, args),
        Boards(args) => cmd_boards(&vfs, args),
        Inspect(args) => cmd_inspect(&vfs, args),
        Repl(args) => cmd_repl(&vfs, args),
        Shots(ShotsCommands::Download(args)) => cmd_shots_download(&vfs, args),
        Key(command) => match command {
            KeyCommands::New(args) => cmd_key_new(&vfs, args),
            KeyCommands::Add(args) => cmd_key_add(&vfs, args),
            KeyCommands::Pub(args) => cmd_key_pub(&vfs, args),
            KeyCommands::Priv(args) => cmd_key_priv(&vfs, args),
            KeyCommands::Rm(args) => cmd_key_rm(&vfs, args),
        },
        Catalog(command) => match command {
            CatalogCommands::List(args) => cmd_catalog_list(args),
            CatalogCommands::Show(args) => cmd_catalog_show(args),
        },
        Name(command) => match command {
            NameCommands::Get => cmd_name_get(&vfs),
            NameCommands::Set(args) => cmd_name_set(&vfs, args),
            NameCommands::Generate => cmd_name_generate(&vfs),
        },
        Runtime(root_args) => match &root_args.command {
            RuntimeCommands::Launch(args) => cmd_launch(root_args, args),
            RuntimeCommands::Restart => cmd_restart(root_args),
            RuntimeCommands::Exit => cmd_exit(root_args),
            RuntimeCommands::Id => cmd_id(root_args),
            RuntimeCommands::Screenshot => cmd_screenshot(root_args),
            RuntimeCommands::Cheat(args) => cmd_cheat(root_args, args),
            RuntimeCommands::Monitor => cmd_monitor(root_args),
            RuntimeCommands::Logs => cmd_logs(root_args),
        },
        Vfs => cmd_vfs(),
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
