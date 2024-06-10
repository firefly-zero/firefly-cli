use anyhow::Context;
use directories::ProjectDirs;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
    let mut rng = thread_rng();

    let adjs = include_str!("names_adj.txt");
    let adjs: Vec<_> = adjs.split_whitespace().collect();
    let adj = adjs.choose(&mut rng).unwrap();

    let nouns = include_str!("names_noun.txt");
    let nouns: Vec<_> = nouns.split_whitespace().collect();
    let noun = nouns.choose(&mut rng).unwrap();

    let name = format!("{adj}-{noun}");
    leetify(&name)
}

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
