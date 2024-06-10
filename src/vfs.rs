use anyhow::Context;
use directories::ProjectDirs;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(clippy::unnecessary_wraps)]
pub fn cmd_vfs() -> anyhow::Result<()> {
    let path = get_vfs_path();
    let path = path.to_str().unwrap();
    println!("{path}");
    Ok(())
}

pub fn get_vfs_path() -> PathBuf {
    match ProjectDirs::from("com", "firefly", "firefly") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => match std::env::current_dir() {
            // Make the path absolute if possible
            Ok(current_dir) => current_dir.join(".firefly"),
            Err(_) => PathBuf::from(".firefly"),
        },
    }
}

pub fn init_vfs() -> anyhow::Result<()> {
    let path = get_vfs_path();
    init_vfs_at(&path)
}

pub fn init_vfs_at(path: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(path.join("roms")).context("create roms directory")?;
    fs::create_dir_all(path.join("sys")).context("create sys directory")?;
    fs::create_dir_all(path.join("data")).context("create data directory")?;

    // Generate random device name if the name file doesn't exist yet.
    let name_path = path.join("sys").join("name");
    if !name_path.exists() {
        let name = generate_name();
        println!("new device name: {name}");
        fs::write(name_path, name).context("write name file")?;
    }

    Ok(())
}

/// Generate a random device name.
fn generate_name() -> String {
    let adj = get_random_line(include_str!("names_adj.txt"));
    let noun = get_random_line(include_str!("names_noun.txt"));
    let name = format!("{adj}-{noun}");
    leetify(&name)
}

/// Select a random line from the given text.
fn get_random_line(adjs: &str) -> &str {
    let mut rng = thread_rng();
    debug_assert!(adjs.ends_with('\n'));
    let adjs = &adjs[..(adjs.len() - 1)];
    let adjs: Vec<_> = adjs.split_whitespace().collect();
    adjs.choose(&mut rng).unwrap()
}

/// Make the given text a bit more 1337-speak.
///
/// It replaces with 50% chance every character than can be made leet.
fn leetify(s: &str) -> String {
    let mut rng = thread_rng();
    let mut replaces = HashMap::new();
    replaces.insert('l', '1');
    replaces.insert('e', '3');
    replaces.insert('a', '4');
    replaces.insert('s', '5');
    replaces.insert('b', '6');
    replaces.insert('t', '7');
    replaces.insert('o', '0');
    let mut res = String::new();
    for c in s.chars() {
        if rng.gen_bool(0.5) {
            res.push(c);
            continue;
        }
        let new = replaces.get(&c).unwrap_or(&c);
        res.push(*new);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_vfs_at() {
        let path = std::env::temp_dir().join("test_init_vfs_at");
        _ = std::fs::remove_dir_all(&path);
        assert!(!path.exists());
        init_vfs_at(&path).unwrap();
        assert_eq!(path.read_dir().unwrap().count(), 3);
        assert!(path.join("sys").metadata().unwrap().is_dir());
        assert!(path.join("roms").metadata().unwrap().is_dir());
        assert!(path.join("data").metadata().unwrap().is_dir());
        assert_eq!(path.join("roms").read_dir().unwrap().count(), 0);
        assert_eq!(path.join("data").read_dir().unwrap().count(), 0);
        assert_eq!(path.join("sys").read_dir().unwrap().count(), 1);
        let name_path = path.join("sys").join("name");
        let name = std::fs::read_to_string(name_path).unwrap();
        assert!(name.contains('-'));
        assert!(name.len() >= 7);
        assert!(name.len() <= 15);
    }

    #[test]
    fn test_generate_name() {
        for _ in 0..1000 {
            let name = generate_name();
            assert!(name.contains('-'));
            assert!(name.len() >= 7);
            assert!(name.len() <= 15);
            assert!(name.is_ascii());
            assert!(!name.starts_with('-'));
            assert!(!name.ends_with('-'));
            for c in name.chars() {
                assert!(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            }
        }
    }

    #[test]
    fn test_leetify() {
        for _ in 0..10 {
            let s = leetify("a");
            assert!(s == "a" || s == "4");
            let s = leetify("s");
            assert!(s == "s" || s == "5");
            let s = leetify("sg");
            assert!(s == "sg" || s == "5g");
            let raw = "the-quick-brown-fox-jumps-over-the-lazy-dog1234567890";
            assert_eq!(leetify(raw).len(), raw.len());
        }
    }
}
