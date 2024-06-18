use crate::args::ImportArgs;
use crate::crypto::hash_dir;
use crate::file_names::{HASH, KEY, META, SIG};
use crate::vfs::{get_vfs_path, init_vfs};
use anyhow::{bail, Context, Result};
use data_encoding::HEXLOWER;
use firefly_meta::Meta;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::signature::hazmat::PrehashVerifier;
use rsa::RsaPublicKey;
use serde::Deserialize;
use sha2::Sha256;
use std::env::temp_dir;
use std::fs::{self, create_dir_all, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// API response from the firefly catalog.
///
/// Example: <https://catalog.fireflyzero.com/sys.launcher.json>
#[derive(Deserialize)]
struct CatalogApp {
    download: String,
}

pub fn cmd_import(args: &ImportArgs) -> Result<()> {
    let path = fetch_archive(&args.path).context("download ROM archive")?;
    let file = File::open(path).context("open archive file")?;
    let mut archive = ZipArchive::new(file).context("open archive")?;

    let meta_raw = read_meta_raw(&mut archive)?;
    let meta = Meta::decode(&meta_raw).context("parse meta")?;
    let vfs_path = get_vfs_path();
    let rom_path = vfs_path.join("roms").join(meta.author_id).join(meta.app_id);

    init_vfs().context("init VFS")?;
    _ = fs::remove_dir_all(&rom_path);
    create_dir_all(&rom_path).context("create ROM dir")?;
    archive.extract(&rom_path).context("extract archive")?;
    if let Err(err) = verify(&rom_path) {
        println!("⚠️  verification failed: {err}");
    }
    if let Some(rom_path) = rom_path.to_str() {
        println!("✅ installed: {rom_path}");
    }
    write_installed(&meta, &vfs_path)?;
    Ok(())
}

fn fetch_archive(path: &str) -> Result<PathBuf> {
    let mut path = path.to_string();
    if path == "launcher" {
        path = "https://github.com/firefly-zero/firefly-launcher/releases/latest/download/sys.launcher.zip".to_string();
    }

    // App ID is given. Fetch download URL from the catalog.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if !path.ends_with(".zip") {
        let url = format!("https://catalog.fireflyzero.com/{path}.json");
        let resp = ureq::get(&url).call().context("send HTTP request")?;
        if resp.status() == 200 && resp.header("Content-Type") == Some("application/json") {
            let app: CatalogApp =
                serde_json::from_reader(&mut resp.into_reader()).context("parse JSON")?;
            path = app.download;
        }
    }

    // Local path is given. Just use it.
    if !path.starts_with("https://") {
        return Ok(path.into());
    }

    // URL is given. Download into a temporary file.
    println!("⏳️ downloading the file...");
    let resp = ureq::get(&path).call().context("send HTTP request")?;
    let out_path = temp_dir().join("rom.zip");
    let mut file = File::create(&out_path)?;
    std::io::copy(&mut resp.into_reader(), &mut file).context("write response into a file")?;
    println!("⌛ installing...");
    Ok(out_path)
}

fn read_meta_raw(archive: &mut ZipArchive<File>) -> Result<Vec<u8>> {
    let mut meta_raw = Vec::new();
    let mut meta_file = archive.by_name(META).context("open meta")?;
    meta_file.read_to_end(&mut meta_raw).context("read meta")?;
    if meta_raw.is_empty() {
        bail!("meta is empty");
    }
    Ok(meta_raw)
}

/// Write the latest installed app name into internal DB.
fn write_installed(meta: &Meta, vfs_path: &Path) -> anyhow::Result<()> {
    let short_meta = firefly_meta::ShortMeta {
        app_id:    meta.app_id,
        author_id: meta.author_id,
    };
    let mut buf = vec![0; short_meta.size()];
    let encoded = short_meta.encode(&mut buf).context("serialize")?;
    let output_path = vfs_path.join("sys").join("new-app");
    fs::write(output_path, &encoded).context("write new-app file")?;
    if meta.launcher {
        let output_path = vfs_path.join("sys").join("launcher");
        fs::write(output_path, &encoded).context("write launcher file")?;
    }
    Ok(())
}

/// Verify SHA256 hash, public key, and signature.
fn verify(rom_path: &Path) -> anyhow::Result<()> {
    let hash_path = rom_path.join(HASH);
    let hash_expected: &[u8] = &fs::read(hash_path).context("read hash file")?;
    let hash_actual: &[u8] = &hash_dir(rom_path).context("calculate hash")?;
    if hash_actual != hash_expected {
        let exp = HEXLOWER.encode(hash_expected);
        let act = HEXLOWER.encode(hash_actual);
        bail!("invalid hash:\n  expected: {exp}\n  got:      {act}");
    }

    let key_path = rom_path.join(KEY);
    let key_raw = fs::read(key_path).context("read key from ROM")?;
    let public_key = RsaPublicKey::from_pkcs1_der(&key_raw).context("decode key")?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);

    let sig_path = rom_path.join(SIG);
    let sig_raw: &[u8] = &fs::read(sig_path).context("read signature")?;
    let sig = Signature::try_from(sig_raw).context("bad signature")?;

    verifying_key
        .verify_prehash(hash_actual, &sig)
        .context("verify signature")?;
    Ok(())
}
