use rand::Rng;
use std::path::PathBuf;

pub fn make_tmp_vfs() -> PathBuf {
    let mut rng = rand::thread_rng();
    let n = rng.gen_range(0..100_000);
    let root = std::env::temp_dir().join(format!("firefly-cli-test-{n}"));
    _ = std::fs::remove_dir_all(&root);
    let vfs = root.join(".firefly");
    std::fs::create_dir_all(&vfs).unwrap();
    vfs
}

pub fn make_tmp_dir() -> PathBuf {
    let mut rng = rand::thread_rng();
    let n = rng.gen_range(0..100_000);
    let root = std::env::temp_dir().join(format!("firefly-cli-test-{n}"));
    _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}
