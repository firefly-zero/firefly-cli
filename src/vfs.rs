use anyhow::Context;
use directories::ProjectDirs;
use firefly_types::Encode;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_vfs_path() -> PathBuf {
    let current_dir = std::env::current_dir().ok();
    if let Some(current_dir) = &current_dir {
        let path = current_dir.join(".firefly");
        if path.is_dir() {
            return path;
        }
    }
    match ProjectDirs::from("com", "firefly", "firefly") {
        Some(dirs) => dirs.data_dir().to_owned(),
        None => match current_dir {
            // Make the path absolute if possible
            Some(current_dir) => current_dir.join(".firefly"),
            None => PathBuf::from(".firefly"),
        },
    }
}

pub fn init_vfs(path: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(path.join("roms")).context("create roms directory")?;
    fs::create_dir_all(path.join("sys").join("pub")).context("create sys/pub directory")?;
    fs::create_dir_all(path.join("sys").join("priv")).context("create sys/priv directory")?;
    fs::create_dir_all(path.join("data")).context("create data directory")?;
    let settings_path = path.join("sys").join("config");
    if !settings_path.exists() {
        let mut settings = firefly_types::Settings {
            xp: 0,
            badges: 0,
            lang: [b'e', b'n'],
            name: generate_valid_name(),
            timezone: detect_tz(),
        };
        if !settings.timezone.contains('/') {
            settings.timezone = "Europe/Amsterdam".to_string();
        }
        let encoded = settings.encode_vec().context("serialize settings")?;
        fs::write(settings_path, encoded).context("write settings file")?;
    }

    Ok(())
}

/// Generate a random valid device name.
pub fn generate_valid_name() -> String {
    loop {
        let name = generate_name();
        if firefly_types::validate_id(&name).is_ok() {
            return name;
        }
    }
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

fn detect_tz() -> String {
    if let Some(tz) = detect_tz_environ() {
        return tz;
    }
    if let Some(tz) = detect_tz_systemctl() {
        return tz;
    }
    "Europe/Amsterdam".to_string()
}

fn detect_tz_environ() -> Option<String> {
    std::env::var("TZ").ok()
}

fn detect_tz_systemctl() -> Option<String> {
    let output = Command::new("timedatectl").arg("show").output().ok()?;
    let stdout = std::str::from_utf8(&output.stdout[..]).ok()?;
    let first_line = stdout.split('\n').next()?;
    let (_, tz) = first_line.split_once('=')?;
    Some(tz.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_vfs_path() {
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..100_000);
        let root = std::env::temp_dir().join(format!("test_get_vfs_path-{n}"));
        std::fs::create_dir_all(&root).unwrap();
        let expected = root.join(".firefly");
        _ = std::fs::remove_dir(&expected);
        std::env::set_current_dir(&root).unwrap();

        let actual = get_vfs_path();
        assert!(actual != expected);

        std::fs::create_dir_all(&expected).unwrap();
        let actual = get_vfs_path();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_init_vfs_at() {
        let path = std::env::temp_dir().join("test_init_vfs_at");
        _ = std::fs::remove_dir_all(&path);
        assert!(!path.exists());
        init_vfs(&path).unwrap();
        assert_eq!(path.read_dir().unwrap().count(), 3);
        assert!(path.join("sys").metadata().unwrap().is_dir());
        assert!(path.join("roms").metadata().unwrap().is_dir());
        assert!(path.join("data").metadata().unwrap().is_dir());
        assert_eq!(path.join("roms").read_dir().unwrap().count(), 0);
        assert_eq!(path.join("data").read_dir().unwrap().count(), 0);
        assert_eq!(path.join("sys").read_dir().unwrap().count(), 4);
        assert_eq!(path.join("sys").join("priv").read_dir().unwrap().count(), 0);
        assert_eq!(path.join("sys").join("pub").read_dir().unwrap().count(), 0);
        assert!(path.join("sys").join("name").exists());
        assert!(path.join("sys").join("config").exists());
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
