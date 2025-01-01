use crate::args::ImportArgs;
use crate::crypto::hash_dir;
use crate::file_names::{HASH, KEY, META, SIG, STATS};
use crate::vfs::init_vfs;
use anyhow::{bail, Context, Result};
use chrono::Datelike;
use data_encoding::HEXLOWER;
use firefly_types::{Encode, Meta};
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

pub fn cmd_import(vfs: &Path, args: &ImportArgs) -> Result<()> {
    let path = fetch_archive(&args.path).context("download ROM archive")?;
    let file = File::open(path).context("open archive file")?;
    let mut archive = ZipArchive::new(file).context("open archive")?;

    let meta_raw = read_meta_raw(&mut archive)?;
    let meta = Meta::decode(&meta_raw).context("parse meta")?;
    let rom_path = vfs.join("roms").join(meta.author_id).join(meta.app_id);

    init_vfs(vfs).context("init VFS")?;
    _ = fs::remove_dir_all(&rom_path);
    create_dir_all(&rom_path).context("create ROM dir")?;
    archive.extract(&rom_path).context("extract archive")?;
    if let Err(err) = verify(&rom_path) {
        println!("⚠️  verification failed: {err}");
    }
    write_stats(&meta, vfs).context("create app stats file")?;
    if let Some(rom_path) = rom_path.to_str() {
        println!("✅ installed: {rom_path}");
    }
    write_installed(&meta, vfs)?;
    Ok(())
}

fn fetch_archive(path: &str) -> Result<PathBuf> {
    let mut path = path.to_string();
    if path == "launcher" {
        path = "https://github.com/firefly-zero/firefly-launcher/releases/latest/download/sys.launcher.zip".to_string();
    }

    // App ID is given. Fetch download URL from the catalog.
    #[expect(clippy::case_sensitive_file_extension_comparisons)]
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
    let mut meta_file = if archive.index_for_name(META).is_some() {
        archive.by_name(META).context("open meta")?
    } else {
        archive.by_name("meta").context("open meta")?
    };
    meta_file.read_to_end(&mut meta_raw).context("read meta")?;
    if meta_raw.is_empty() {
        bail!("meta is empty");
    }
    Ok(meta_raw)
}

/// Write the latest installed app name into internal DB.
fn write_installed(meta: &Meta<'_>, vfs_path: &Path) -> anyhow::Result<()> {
    let short_meta = firefly_types::ShortMeta {
        app_id: meta.app_id,
        author_id: meta.author_id,
    };
    let encoded = short_meta.encode_vec().context("serialize")?;
    let output_path = vfs_path.join("sys").join("new-app");
    fs::write(output_path, &encoded).context("write new-app file")?;
    if meta.launcher {
        let output_path = vfs_path.join("sys").join("launcher");
        fs::write(output_path, encoded).context("write launcher file")?;
    }
    Ok(())
}

/// Verify SHA256 hash, public key, and signature.
fn verify(rom_path: &Path) -> anyhow::Result<()> {
    let hash_path = rom_path.join(HASH);
    let hash_expected: &[u8] = &fs::read(hash_path).context("read hash file")?;
    let hash_actual: &[u8] = &hash_dir(rom_path).context("calculate hash")?[..];
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

/// Create or update app stats based on the default stats file.
fn write_stats(meta: &Meta<'_>, vfs_path: &Path) -> anyhow::Result<()> {
    let data_path = vfs_path.join("data").join(meta.author_id).join(meta.app_id);
    if !data_path.exists() {
        fs::create_dir_all(&data_path).context("create data dir")?;
    }
    let stats_path = data_path.join("stats");
    let rom_path = vfs_path.join("roms").join(meta.author_id).join(meta.app_id);
    let default_path = rom_path.join(STATS);
    if stats_path.exists() {
        update_stats(&default_path, &stats_path)?;
    } else {
        copy_stats(&default_path, &stats_path)?;
    }
    Ok(())
}

fn copy_stats(default_path: &Path, stats_path: &Path) -> anyhow::Result<()> {
    let raw = fs::read(default_path).context("read default stats file")?;
    let default = firefly_types::Stats::decode(&raw)?;
    let today = chrono::Local::now().date_naive();
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let today = (
        today.year() as u16,
        today.month0() as u8,
        today.day0() as u8,
    );
    let stats = firefly_types::Stats {
        minutes: [0; 4],
        longest_play: [0; 4],
        launches: [0; 4],
        installed_on: today,
        updated_on: today,
        launched_on: (0, 0, 0),
        xp: 0,
        badges: default.badges,
        scores: default.scores,
    };
    let raw = stats.encode_vec().context("encode stats")?;
    fs::write(stats_path, raw).context("write stats file")?;
    Ok(())
}

fn update_stats(default_path: &Path, stats_path: &Path) -> anyhow::Result<()> {
    let raw = fs::read(stats_path).context("read stats file")?;
    let old_stats = firefly_types::Stats::decode(&raw).context("parse old stats")?;

    let raw = fs::read(default_path).context("read default stats file")?;
    let default = firefly_types::Stats::decode(&raw)?;

    let today = chrono::Local::now().date_naive();
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let today = (
        today.year() as u16,
        today.month0() as u8,
        today.day0() as u8,
    );
    // The current date might be behind the current date on the device,
    // and it might be reflected in the dates recorded in the stats.
    // If that happens, try to stay closer to the device time.
    let today = today
        .max(old_stats.installed_on)
        .max(old_stats.launched_on)
        .max(old_stats.updated_on);

    let mut badges = Vec::new();
    for (i, default_badge) in default.badges.iter().enumerate() {
        let new_badge = if let Some(old_badge) = old_stats.badges.get(i) {
            firefly_types::BadgeProgress {
                new: old_badge.new,
                done: old_badge.done.min(default_badge.goal),
                goal: default_badge.goal,
            }
        } else {
            firefly_types::BadgeProgress {
                new: false,
                done: 0,
                goal: default_badge.goal,
            }
        };
        badges.push(new_badge);
    }

    let mut scores = Vec::from(old_stats.scores);
    scores.truncate(default.scores.len());
    for _ in scores.len()..default.scores.len() {
        let fs = firefly_types::FriendScore { index: 0, score: 0 };
        let new_score = firefly_types::BoardScores {
            me: Box::new([0i16; 8]),
            friends: Box::new([fs; 8]),
        };
        scores.push(new_score);
    }

    let new_stats = firefly_types::Stats {
        minutes: old_stats.minutes,
        longest_play: old_stats.longest_play,
        launches: old_stats.launches,
        installed_on: old_stats.installed_on,
        updated_on: today,
        launched_on: old_stats.launched_on,
        xp: old_stats.xp.min(1000),
        badges: badges.into_boxed_slice(),
        scores: scores.into_boxed_slice(),
    };
    let raw = new_stats.encode_vec().context("encode updated stats")?;
    fs::write(stats_path, raw).context("write updated stats file")?;
    Ok(())
}
