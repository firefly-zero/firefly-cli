use directories::ProjectDirs;
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
        None => PathBuf::from(".firefly"),
    }
}
