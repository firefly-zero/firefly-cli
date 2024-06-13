use crate::file_names::{HASH, KEY, SIG};
use anyhow::Context;
use sha2::digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub fn hash_dir(rom_path: &Path) -> anyhow::Result<GenericArray<u8, U32>> {
    // generate one big hash for all files
    let mut hasher = Sha256::new();
    let files = rom_path.read_dir().context("open the ROM dir")?;
    let mut file_paths = Vec::new();
    for entry in files {
        let entry = entry.context("access dir entry")?;
        file_paths.push(entry.path());
    }
    file_paths.sort();
    for path in file_paths {
        let file_name = path.file_name().context("get file name")?;
        if file_name == HASH || file_name == SIG || file_name == KEY {
            continue;
        }
        hasher.update("\x00");
        hasher.update(file_name.as_bytes());
        hasher.update("\x00");
        let mut file = std::fs::File::open(path).context("open file")?;
        std::io::copy(&mut file, &mut hasher).context("read file")?;
    }

    // write the hash into a file
    let hash = hasher.finalize();
    Ok(hash)
}
