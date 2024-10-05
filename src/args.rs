#![allow(clippy::module_name_repetitions)]

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the project and install it locally (into VFS).
    Build(BuildArgs),

    /// Export an installed app as a zip archive.
    Export(ExportArgs),

    /// Install locally an app from a zip archive.
    #[clap(alias("install"))]
    Import(ImportArgs),

    /// Show the full path to the virtual filesystem.
    Vfs,

    /// Send a cheat code into a running game.
    Cheat(CheatArgs),

    /// Show runtime stats for a running device (or emulator).
    Monitor(MonitorArgs),

    /// Inspect contents of the ROM: files, metadata, wasm binary.
    Inspect(InspectArgs),

    /// Run interactive session.
    Repl(ReplArgs),

    /// Commands to manage signing keys.
    #[command(subcommand)]
    #[clap(alias("keys"))]
    Key(KeyCommands),

    /// Commands to set, get, and generate device name.
    #[command(subcommand)]
    Name(NameCommands),

    /// Commands to interact with catalog.fireflyzero.com.
    #[command(subcommand)]
    Catalog(CatalogCommands),
}

#[derive(Subcommand, Debug)]
pub enum KeyCommands {
    /// Generate a new key pair.
    #[clap(alias("gen"), alias("generate"))]
    New(KeyArgs),

    /// Add a new key from catalog, URL, or file.
    #[clap(alias("import"))]
    Add(KeyArgs),

    /// Export public key.
    #[clap(alias("export"), alias("public"))]
    Pub(KeyExportArgs),

    /// Export private key.
    #[clap(alias("private"))]
    Priv(KeyExportArgs),

    /// Remove the public and private key.
    #[clap(alias("remove"))]
    Rm(KeyArgs),
}

#[derive(Subcommand, Debug)]
pub enum NameCommands {
    /// Show the current device name.
    #[clap(alias("show"), alias("echo"))]
    Get(NameGetArgs),

    /// Set a new device name.
    #[clap(alias("change"))]
    Set(NameSetArgs),

    /// Set a new device name.
    #[clap(alias("gen"), alias("new"))]
    Generate(NameGenerateArgs),
}

#[derive(Subcommand, Debug)]
pub enum CatalogCommands {
    /// List all games available in the catalog.
    #[clap(alias("ls"), alias("apps"))]
    List(CatalogListArgs),

    /// Show info about an app or author.
    #[clap(alias("info"), alias("app"), alias("author"))]
    Show(CatalogShowArgs),
}

#[derive(Debug, Parser)]
pub struct KeyArgs {
    pub author_id: String,
}

#[derive(Debug, Parser)]
pub struct KeyExportArgs {
    pub author_id: String,

    /// Path to the exported key file.
    #[arg(short, long, default_value = None)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct NameGetArgs {}

#[derive(Debug, Parser)]
pub struct NameSetArgs {
    pub name: String,
}

#[derive(Debug, Parser)]
pub struct NameGenerateArgs {}

#[derive(Debug, Parser)]
pub struct BuildArgs {
    /// Path to the project root.
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Path to the directory where to store roms
    #[arg(short, long, default_value = None)]
    pub roms: Option<PathBuf>,

    /// Path to the firefly config.
    #[arg(short, long, default_value = None)]
    pub config: Option<PathBuf>,

    /// Don't optimize the binary.
    #[arg(long, default_value_t = false)]
    pub no_opt: bool,

    /// Don't strip debug info and custom sections.
    #[arg(long, default_value_t = false)]
    pub no_strip: bool,

    /// Don't show a random tip.
    #[arg(long, default_value_t = false)]
    pub no_tip: bool,
}

#[derive(Debug, Parser)]
pub struct ExportArgs {
    /// Path to the project root.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Full app ID.
    #[arg(long, default_value = None)]
    pub id: Option<String>,

    /// Path to the archive.
    #[arg(short, long, default_value = None)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ImportArgs {
    /// The ROM to install.
    ///
    /// The ROM can be one of:
    ///
    /// 1. Local path to a zip file (for example, `sys.launcher.zip`)
    ///
    /// 2. URL of a zip file (for example, `https://example.com/sys.launcher.zip`)
    ///
    /// 3. App ID in the catalog (for example, `sys.launcher`).
    ///
    /// 4. The word "launcher" to install the latest version of the default launcher.
    #[arg()]
    pub path: String,
}

#[derive(Debug, Parser)]
pub struct MonitorArgs {}

#[derive(Debug, Parser)]
pub struct InspectArgs {
    /// ID of the ROM to inspect.
    ///
    /// If not specified, the ID of the current project is used.
    #[arg(default_value = None)]
    pub id: Option<String>,

    /// Path to the project root.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
}

#[derive(Debug, Parser)]
pub struct CheatArgs {
    /// The command to pass into the app.
    ///
    /// Either an integer or a command listed in firefly.toml.
    #[arg()]
    pub command: String,

    /// The value to pass into the app.
    ///
    /// Either an integer, boolean, or a character.
    #[arg()]
    pub value: String,

    /// Path to the project root.
    #[arg(default_value = ".")]
    pub root: PathBuf,
}

#[derive(Debug, Parser)]
pub struct ReplArgs {
    /// Path to the project root.
    #[arg(default_value = ".")]
    pub root: PathBuf,
}

#[derive(Debug, Parser)]
pub struct CatalogListArgs {
    // TODO(@orsinium): support JSON
}

#[derive(Debug, Parser)]
pub struct CatalogShowArgs {
    /// The app/author ID to get info for. For example, "lux.snek".
    #[arg()]
    pub id: String,
}
