use crate::args::{KeyArgs, KeyExportArgs};
use crate::vfs::{get_vfs_path, init_vfs};
use anyhow::{bail, Context};
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn cmd_key_new(args: &KeyArgs) -> anyhow::Result<()> {
    init_vfs().context("init vfs")?;
    let vfs_path = get_vfs_path();

    let author = &args.author_id;
    if let Err(err) = firefly_meta::validate_id(author) {
        bail!("invalid author ID: {err}")
    }

    // generate and check paths for keys
    let sys_path = vfs_path.join("sys");
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
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).context("generate key")?;
    println!("⌛ saving keys...");
    let mut priv_file = fs::File::create(priv_path)?;
    let priv_bytes = priv_key.to_pkcs1_der().context("serialize priv key")?;
    priv_file.write_all(priv_bytes.as_bytes())?;

    // save public key
    let pub_key = RsaPublicKey::from(&priv_key);
    let mut pub_file = fs::File::create(pub_path)?;
    let pub_bytes = pub_key.to_pkcs1_der().context("serialize pub key")?;
    pub_file.write_all(pub_bytes.as_bytes())?;

    println!("✅ generated key pair for {author}");
    Ok(())
}

pub fn cmd_key_pub(args: &KeyExportArgs) -> anyhow::Result<()> {
    export_key(args, true)
}

pub fn cmd_key_priv(args: &KeyExportArgs) -> anyhow::Result<()> {
    export_key(args, false)
}

pub fn export_key(args: &KeyExportArgs, public: bool) -> anyhow::Result<()> {
    let author = &args.author_id;
    let output_path = match &args.output {
        Some(output) => output,
        None => &PathBuf::new().join(format!("{author}.der")),
    };
    let key_type = if public { "public" } else { "private" };

    // export the key file
    {
        let part = if public { "pub" } else { "priv" };
        let vfs_path = get_vfs_path();
        let key_path = vfs_path.join("sys").join(part).join(author);
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

pub fn cmd_key_rm(args: &KeyArgs) -> anyhow::Result<()> {
    let vfs_path = get_vfs_path();

    let author = &args.author_id;
    if let Err(err) = firefly_meta::validate_id(author) {
        bail!("invalid author ID: {err}")
    }

    // generate and check paths for keys
    let sys_path = vfs_path.join("sys");
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
