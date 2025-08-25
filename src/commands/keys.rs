use crate::args::{KeyArgs, KeyExportArgs};
use crate::vfs::init_vfs;
use anyhow::{bail, Context};
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(test)]
const BIT_SIZE: usize = 128;

#[cfg(not(test))]
const BIT_SIZE: usize = 2048;

pub fn cmd_key_new(vfs: &Path, args: &KeyArgs) -> anyhow::Result<()> {
    init_vfs(vfs).context("init vfs")?;
    let author = &args.author_id;
    if let Err(err) = firefly_types::validate_id(author) {
        bail!("invalid author ID: {err}")
    }

    // generate and check paths for keys
    let sys_path = vfs.join("sys");
    let priv_path = sys_path.join("priv").join(author);
    let pub_path = sys_path.join("pub").join(author);
    if priv_path.exists() {
        bail!("the key pair for {author} already exists")
    }
    if pub_path.exists() {
        bail!("the public key for {author} already exists")
    }

    // generate and save private key
    let mut rng = rand::thread_rng();
    println!("⏳️ generating key pair...");
    let priv_key = RsaPrivateKey::new(&mut rng, BIT_SIZE).context("generate key")?;
    println!("⌛ saving keys...");
    let mut priv_file = fs::File::create(priv_path)?;
    let priv_bytes = priv_key.to_pkcs1_der().context("serialize priv key")?;
    priv_file
        .write_all(priv_bytes.as_bytes())
        .context("write priv key")?;

    // save public key
    let pub_key = RsaPublicKey::from(&priv_key);
    let mut pub_file = fs::File::create(pub_path)?;
    let pub_bytes = pub_key.to_pkcs1_der().context("serialize pub key")?;
    pub_file
        .write_all(pub_bytes.as_bytes())
        .context("write pub key")?;

    println!("✅ generated key pair for {author}");
    Ok(())
}

pub fn cmd_key_pub(vfs: &Path, args: &KeyExportArgs) -> anyhow::Result<()> {
    export_key(vfs, args, true)
}

pub fn cmd_key_priv(vfs: &Path, args: &KeyExportArgs) -> anyhow::Result<()> {
    export_key(vfs, args, false)
}

pub fn export_key(vfs: &Path, args: &KeyExportArgs, public: bool) -> anyhow::Result<()> {
    let author = &args.author_id;
    let output_path = match &args.output {
        Some(output) => output,
        None => &PathBuf::new().join(format!("{author}.der")),
    };
    if output_path.is_dir() {
        bail!("the --output path must be a file, not directory");
    }
    if output_path.exists() {
        bail!("the --output path already exists");
    }
    let key_type = if public { "public" } else { "private" };

    // export the key file
    {
        let part = if public { "pub" } else { "priv" };
        let key_path = vfs.join("sys").join(part).join(author);
        if !key_path.exists() {
            bail!("{key_type} key for {author} not found");
        }
        fs::copy(key_path, output_path).context("copy key")?;
    }

    // make the file read-only (if possible)
    {
        let meta = fs::metadata(output_path).context("get file metadata")?;
        let mut perms = meta.permissions();
        perms.set_readonly(true);
        _ = fs::set_permissions(output_path, perms);
    }

    let output_path = output_path.to_str().unwrap_or("the output path");
    println!("✅ the {key_type} key saved into {output_path}");
    Ok(())
}

pub fn cmd_key_rm(vfs: &Path, args: &KeyArgs) -> anyhow::Result<()> {
    let author = &args.author_id;
    if let Err(err) = firefly_types::validate_id(author) {
        bail!("invalid author ID: {err}")
    }

    // generate and check paths for keys
    let sys_path = vfs.join("sys");
    let priv_path = sys_path.join("priv").join(author);
    let pub_path = sys_path.join("pub").join(author);
    let mut found = true;
    if priv_path.exists() {
        fs::remove_file(priv_path)?;
    } else {
        println!("⚠️  private key not found");
        found = false;
    }
    if pub_path.exists() {
        fs::remove_file(pub_path)?;
    } else {
        println!("⚠️  public key not found");
        found = false;
    }
    if found {
        println!("✅ key pair is removed");
    }
    Ok(())
}

pub fn cmd_key_add(vfs: &Path, args: &KeyArgs) -> anyhow::Result<()> {
    init_vfs(vfs).context("init vfs")?;
    let key_path = &args.author_id;
    let (author, raw_key) = if key_path.starts_with("https://") {
        println!("⏳️ downloading the key from URL...");
        download_key(key_path)?
    } else if PathBuf::from(key_path).exists() {
        println!("⏳️ reading the key from file...");
        let key_path = PathBuf::from(key_path);
        let author = key_path.file_stem().context("get file name")?;
        let author = author.to_str().context("convert file name to UTF-8")?;
        let author = author.to_string();
        let key_raw = fs::read(&key_path)?;
        (author, key_raw)
    } else if firefly_types::validate_id(key_path).is_ok() {
        println!("⏳️ downloading the key from catalog...");
        let url = format!("https://catalog.fireflyzero.com/keys/{key_path}.der");
        download_key(&url)?
    } else {
        bail!("the key file not found")
    };
    save_raw_key(vfs, &author, &raw_key)?;
    println!("✅ added new key");
    Ok(())
}

/// Download the key from the given URL.
fn download_key(url: &str) -> anyhow::Result<(String, Vec<u8>)> {
    let file_name = url.split('/').next_back().unwrap();
    let Some(author) = file_name.strip_suffix(".der") else {
        bail!("the key file must have .der extension")
    };
    let resp = ureq::get(url).call().context("download the key")?;
    let buf = resp.into_body().read_to_vec()?;
    Ok((author.to_string(), buf))
}

/// Save the given key into VFS.
fn save_raw_key(vfs: &Path, author: &str, raw_key: &[u8]) -> anyhow::Result<()> {
    if raw_key.len() < 20 {
        bail!("the key is too small")
    }
    if raw_key.len() > 2048 {
        bail!("the key is too big")
    }
    let sys_path = vfs.join("sys");
    let pub_path = sys_path.join("pub").join(author);
    if let Ok(key) = RsaPrivateKey::from_pkcs1_der(raw_key) {
        let path = sys_path.join("priv").join(author);
        fs::write(path, raw_key).context("write private key")?;

        // generate and save public key
        let key = key.to_public_key();
        let pub_der = key.to_pkcs1_der().context("extract public key")?;
        let pub_raw = pub_der.as_bytes();
        fs::write(pub_path, pub_raw).context("write public part of the key")?;
    } else {
        RsaPublicKey::from_pkcs1_der(raw_key).context("parse public key")?;
        fs::write(pub_path, raw_key).context("write public key")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_cmd_key_new() {
        let vfs = make_tmp_vfs();
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };
        cmd_key_new(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(key_path.is_file());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(key_path.is_file());
    }

    #[test]
    fn test_cmd_key_pub() {
        let vfs = make_tmp_vfs();
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };
        cmd_key_new(&vfs, &args).unwrap();

        let key_path = vfs.join("greg.der");
        let args = KeyExportArgs {
            author_id: "greg".to_string(),
            output: Some(key_path.clone()),
        };
        cmd_key_pub(&vfs, &args).unwrap();
        assert!(&key_path.is_file());
        let meta = key_path.metadata().unwrap();
        assert_eq!(meta.len(), 26);
    }

    #[test]
    fn test_cmd_key_priv() {
        let vfs = make_tmp_vfs();
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };
        cmd_key_new(&vfs, &args).unwrap();

        let key_path = vfs.join("greg.der");
        let args = KeyExportArgs {
            author_id: "greg".to_string(),
            output: Some(key_path.clone()),
        };
        cmd_key_priv(&vfs, &args).unwrap();
        assert!(&key_path.is_file());
        let meta = key_path.metadata().unwrap();
        let size = meta.len();
        assert!(size >= 99 || size <= 101, "{size} != 100");
    }

    #[test]
    fn test_cmd_key_add_pub() {
        let vfs = make_tmp_vfs();
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };

        // create key
        cmd_key_new(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(key_path.is_file());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(key_path.is_file());

        // export key
        let export_path = vfs.join("greg.der");
        let args = KeyExportArgs {
            author_id: "greg".to_string(),
            output: Some(export_path.clone()),
        };
        cmd_key_pub(&vfs, &args).unwrap();

        // drop key
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };
        cmd_key_rm(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(!key_path.exists());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(!key_path.exists());

        // import key from file
        let args = KeyArgs {
            author_id: export_path.to_str().unwrap().to_string(),
        };
        cmd_key_add(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(!key_path.exists());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(key_path.exists());
    }

    #[test]
    fn test_cmd_key_add_priv() {
        let vfs = make_tmp_vfs();
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };

        // create key
        cmd_key_new(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(key_path.is_file());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(key_path.is_file());

        // export key
        let export_path = vfs.join("greg.der");
        let args = KeyExportArgs {
            author_id: "greg".to_string(),
            output: Some(export_path.clone()),
        };
        cmd_key_priv(&vfs, &args).unwrap();

        // drop key
        let args = KeyArgs {
            author_id: "greg".to_string(),
        };
        cmd_key_rm(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(!key_path.exists());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(!key_path.exists());

        // import key from file
        let args = KeyArgs {
            author_id: export_path.to_str().unwrap().to_string(),
        };
        cmd_key_add(&vfs, &args).unwrap();
        let key_path = vfs.join("sys").join("priv").join("greg");
        assert!(key_path.exists());
        let key_path = vfs.join("sys").join("pub").join("greg");
        assert!(key_path.exists());
    }
}
