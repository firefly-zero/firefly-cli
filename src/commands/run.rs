use crate::args::{BuildArgs, EmulatorArgs, ExportArgs};
use crate::commands::{cmd_build, cmd_emulator, cmd_export};
use crate::config::Config;
use anyhow::{Context, Result};
use std::path::Path;

pub fn cmd_run(vfs: &Path, build_args: &BuildArgs) -> Result<()> {
    let config =
        Config::load(vfs.to_path_buf(), &build_args.root).context("load project config")?;
    let id = format!("{}.{}", config.author_id, config.app_id);

    cmd_build(vfs.to_path_buf(), build_args).context("build")?;

    let emulator_args = EmulatorArgs {
        id: Some(id.clone()),
        ..Default::default()
    };
    cmd_emulator(vfs, &emulator_args).context("emulator")?;

    let export_args = ExportArgs {
        root: build_args.root.clone(),
        id: Some(id),
        output: None,
    };
    cmd_export(vfs, &export_args).context("export")?;
    Ok(())
}
