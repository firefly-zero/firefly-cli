use rand::Rng;
use sha2::digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn make_tmp_vfs() -> PathBuf {
    let mut rng = rand::rng();
    let n = rng.random_range(0..100_000);
    let root = std::env::temp_dir().join(format!("firefly-cli-test-{n}"));
    _ = std::fs::remove_dir_all(&root);
    let vfs = root.join(".firefly");
    std::fs::create_dir_all(&vfs).unwrap();
    vfs
}

pub fn make_tmp_dir() -> PathBuf {
    let mut rng = rand::rng();
    let n = rng.random_range(0..100_000);
    let root = std::env::temp_dir().join(format!("firefly-cli-test-{n}"));
    _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Recursively compare all files in dirs and ensure they are the same
pub fn dirs_eq(dir1: &Path, dir2: &Path) {
    let hashes1 = dir_hashes(dir1);
    let hashes2 = dir_hashes(dir2);
    assert!(!hashes1.is_empty());
    assert_eq!(hashes1.len(), hashes2.len());
    for (path, hash1) in hashes1 {
        let hash2 = hashes2[&path];
        assert_eq!(hash1, hash2, "files at path {path} differ");
    }
}

/// Get a hashsum for every file in the directory.
pub fn dir_hashes(dir: &Path) -> HashMap<String, GenericArray<u8, U32>> {
    let mut results = HashMap::new();
    let entries = fs::read_dir(dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let meta = entry.metadata().unwrap();
        let entry_name = entry.file_name().into_string().unwrap();
        if meta.is_dir() {
            let subhashes = dir_hashes(&entry.path());
            for (file_name, hash) in subhashes {
                let path = format!("{entry_name}/{file_name}");
                results.insert(path, hash);
            }
        } else {
            let mut hasher = Sha256::new();
            let mut file = std::fs::File::open(entry.path()).unwrap();
            std::io::copy(&mut file, &mut hasher).unwrap();
            results.insert(entry_name, hasher.finalize());
        }
    }
    results
}
