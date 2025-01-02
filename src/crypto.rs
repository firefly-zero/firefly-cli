use crate::file_names::{HASH, SIG};
use anyhow::{bail, Context};
use sha2::digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Generate one big hash for all files in the given directory.
pub fn hash_dir(rom_path: &Path) -> anyhow::Result<GenericArray<u8, U32>> {
    let mut hasher = Sha256::new();
    let files = rom_path.read_dir().context("open the ROM dir")?;
    let mut file_paths = Vec::new();
    for entry in files {
        let entry = entry.context("access dir entry")?;
        file_paths.push(entry.path());
    }
    file_paths.sort();
    for path in file_paths {
        if !path.is_file() {
            bail!("the ROM dir must contain only files");
        }
        let file_name = path.file_name().context("get file name")?;
        if file_name == HASH || file_name == SIG {
            continue;
        }
        hasher.update("\x00");
        hasher.update(file_name.as_encoded_bytes());
        hasher.update("\x00");
        let mut file = std::fs::File::open(path).context("open file")?;
        std::io::copy(&mut file, &mut hasher).context("read file")?;
    }
    let hash = hasher.finalize();
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_hash_dir() {
        let dir = make_tmp_dir();
        assert_eq!(dir.read_dir().unwrap().count(), 0);
        std::fs::write(dir.join("somefile"), "hello").unwrap();
        let hash1: &[u8] = &hash_dir(&dir).unwrap()[..];
        let hash2: &[u8] = &hash_dir(&dir).unwrap()[..];
        assert_eq!(hash1, hash2, "not idempotent");

        std::fs::write(dir.join("somefile"), "hell").unwrap();
        let hash3: &[u8] = &hash_dir(&dir).unwrap()[..];
        assert!(hash2 != hash3, "doesn't change if file changed");

        std::fs::write(dir.join("somefile2"), "hell").unwrap();
        let hash4: &[u8] = &hash_dir(&dir).unwrap()[..];
        assert!(hash3 != hash4, "doesn't change if fiels added");
    }
}
