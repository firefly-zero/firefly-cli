use anyhow::Context;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

pub(crate) fn cmd_vfs() -> anyhow::Result<()> {
    let path = get_vfs_path();
    let path = path.to_str().unwrap();
    println!("{path}");
    Ok(())
}

pub(crate) fn get_vfs_path() -> PathBuf {
    match ProjectDirs::from("com", "firefly", "firefly") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => match std::env::current_dir() {
            // Make the path absolute if possible
            Ok(current_dir) => current_dir.join(".firefly"),
            Err(_) => PathBuf::from(".firefly"),
        },
    }
}

pub(crate) fn init_vfs() -> anyhow::Result<()> {
    let path = get_vfs_path();
    fs::create_dir_all(&path).context("create vfs directory")?;
    fs::create_dir_all(path.join("roms")).context("create roms directory")?;
    fs::create_dir_all(path.join("sys")).context("create sys directory")?;
    fs::create_dir_all(path.join("data")).context("create data directory")?;
    Ok(())
}
