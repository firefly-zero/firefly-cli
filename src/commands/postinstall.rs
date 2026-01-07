use anyhow::{Context, Result, bail};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub fn cmd_postinstall() -> Result<()> {
    let path = move_self()?;
    create_alias(&path)?;
    Ok(())
}

/// Move the currently running executable into $PATH.
fn move_self() -> Result<PathBuf> {
    if let Some(path) = find_writable_path() {
        move_self_to(&path)?;
        return Ok(path);
    }
    if let Some(home) = std::env::home_dir() {
        let path = home.join(".local").join("bin");
        if is_writable(&path) {
            move_self_to(&path)?;
            add_path(&path)?;
            return Ok(path);
        }
    }
    bail!("cannot write writable dir in $PATH")
}

/// Move the currently running executable into the given path.
fn move_self_to(new_path: &Path) -> Result<()> {
    let Some(old_path) = std::env::args().next() else {
        bail!("cannot access process args");
    };
    let old_path = PathBuf::from(old_path);
    if !old_path.exists() {
        bail!("the binary is execute not by its path");
    }
    let new_path = new_path.join("firefly_cli");
    std::fs::rename(old_path, new_path).context("move binary")?;
    Ok(())
}

/// Create `ff` shortcut for `firefly_cli`.
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

/// Check if the current user can create files in the given directory.
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

/// Add the given directory into `$PATH`.
fn add_path(path: &Path) -> Result<()> {
    let Some(home) = std::env::home_dir() else {
        bail!("home dir not found");
    };
    let zshrc = home.join(".zshrc");
    if zshrc.exists() {
        return add_path_to(&zshrc, path);
    }
    let bashhrc = home.join(".bashhrc");
    if bashhrc.exists() {
        return add_path_to(&bashhrc, path);
    }
    bail!("cannot find .zshrc or .bashrc")
}

fn add_path_to(profile: &Path, path: &Path) -> Result<()> {
    let mut file = std::fs::OpenOptions::new().append(true).open(profile)?;
    let path_bin = path.as_os_str().as_encoded_bytes();
    file.write_all(b"\n\nexport PATH=\"$PATH:")?;
    file.write_all(path_bin)?;
    file.write_all(b"\"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_paths() {
        let got = parse_paths("/a/b:~/c/:d");
        let exp = vec![
            PathBuf::from("/a/b"),
            PathBuf::from("~/c/"),
            PathBuf::from("d"),
        ];
        assert_eq!(got, exp);
    }

    #[test]
    fn test_is_writable() {
        let path = PathBuf::from(".");
        assert!(is_writable(&path));
    }
}
