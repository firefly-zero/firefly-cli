use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

pub fn cmd_postinstall() -> Result<()> {
    let Some(path) = find_writable_path() else {
        bail!("cannot write writable dir in PATH")
    };
    move_self_to(&path)?;
    create_alias(&path)?;
    Ok(())
}

fn move_self_to(new_path: &Path) -> Result<()> {
    let Some(old_path) = std::env::args().next() else {
        bail!("cannot access process args");
    };
    let old_path = PathBuf::from(old_path);
    let new_path = new_path.join("firefly_cli");
    std::fs::rename(old_path, new_path)?;
    Ok(())
}

fn create_alias(dir_path: &Path) -> Result<()> {
    #[cfg(unix)]
    create_alias_unix(dir_path)?;
    #[cfg(not(unix))]
    println!("⚠️  The `ff` alias can be created only on UNIX systems.");
    Ok(())
}

#[cfg(unix)]
fn create_alias_unix(dir_path: &Path) -> Result<()> {
    let old_path = dir_path.join("firefly_cli");
    let new_path = dir_path.join("ff");
    std::os::unix::fs::symlink(old_path, new_path)?;
    Ok(())
}

/// Find a path in `$PATH` in which the current user can create files.
fn find_writable_path() -> Option<PathBuf> {
    let paths = load_paths();

    // Prefer writable paths in the user home directory.
    if let Some(home) = std::env::home_dir() {
        for path in &paths {
            let in_home = path.starts_with(&home);
            if in_home && is_writable(path) {
                return Some(path.clone());
            }
        }
    }

    // If no writable paths in the home dir, find a writable path naywhere else.
    for path in &paths {
        if is_writable(path) {
            return Some(path.clone());
        }
    }

    // No writable paths in $PATH.
    None
}

fn is_writable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let readonly = meta.permissions().readonly();
    if readonly {
        return false;
    }

    // Even if the dir is not marked as readonly, file writes to it may still fail.
    // So, there is only one way to know for sure.
    let file_path = path.join("_temp-file-by-firefly-cli-pls-delete");
    let res = std::fs::write(&file_path, "");
    _ = std::fs::remove_file(file_path);
    res.is_ok()
}

/// Read and parse paths from `$PATH`.
fn load_paths() -> Vec<PathBuf> {
    let Ok(raw) = std::env::var("PATH") else {
        return Vec::new();
    };
    parse_paths(&raw)
}

fn parse_paths(raw: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    const SEP: char = ';';
    #[cfg(not(windows))]
    const SEP: char = ':';

    let mut paths = Vec::new();
    for path in raw.split(SEP) {
        paths.push(PathBuf::from(path));
    }
    paths
}
