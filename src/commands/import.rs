use crate::args::ImportArgs;
use crate::crypto::hash_dir;
use crate::file_names::{HASH, META, STATS};
use crate::vfs::init_vfs;
use anyhow::{Context, Result, bail};
use chrono::Datelike;
use data_encoding::HEXLOWER;
use firefly_types::{Encode, Meta, validate_id};
use serde::Deserialize;
use std::env::temp_dir;
use std::fs::{self, File, create_dir_all};
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
    if !id_matches(&args.path, &meta) {
        bail!(
            "app ID ({}.{}) doesn't match the expected ID",
            meta.author_id,
            meta.app_id
        );
    }
    if (meta.launcher || meta.sudo) && meta.author_id != "sys" {
        println!("⚠️  The app uses privileged system access. Make sure you trust the author.");
    }
    let rom_path = vfs.join("roms").join(meta.author_id).join(meta.app_id);

    init_vfs(vfs).context("init VFS")?;
    _ = fs::remove_dir_all(&rom_path);
    create_dir_all(&rom_path).context("create ROM dir")?;
    archive.extract(&rom_path).context("extract archive")?;
    if let Err(err) = verify_hash(&rom_path) {
        println!("⚠️  hash verification failed: {err}");
    }
    create_data_dir(&meta, vfs).context("create app data directory")?;
    write_stats(&meta, vfs).context("create app stats file")?;
    if let Some(rom_path) = rom_path.to_str() {
        println!("✅ installed: {rom_path}");
    }
    write_installed(&meta, vfs)?;
    reset_launcher_cache(vfs).context("reset launcher cache")?;
    Ok(())
}

/// Check if the ID from the ID/path/URL that the user provided matches the app ID in meta.
///
/// Currently verifies ID only if the app source is the catalog.
/// For installation from URL/file we let the URL/file to have any name.
fn id_matches(given: &str, meta: &Meta<'_>) -> bool {
    let is_catalog = !given.ends_with(".zip");
    if !is_catalog {
        return true;
    }
    if given == "launcher" {
        return meta.author_id == "sys" && meta.app_id == "launcher";
    }
    let full_id = format!("{}.{}", meta.author_id, meta.app_id);
    given == full_id
}

/// Fetch the given app archive as a file.
///
/// * If file path is given, this path will be returned without any file modification.
/// * If URL is given, the file will be downloaded.
/// * If app ID is given, try downloading the app from the catalog.
fn fetch_archive(path: &str) -> Result<PathBuf> {
    let mut path = path;
    if path == "launcher" {
        path = "https://github.com/firefly-zero/firefly-launcher/releases/latest/download/sys.launcher.zip";
    }
    let mut path = path.to_string();

    // App ID is given. Fetch download URL from the catalog.
    if !path.ends_with(".zip") {
        let Some((author_id, app_id)) = path.split_once('.') else {
            bail!("app ID must contain dot");
        };
        if let Err(err) = validate_id(author_id) {
            bail!("invalid author ID: {err}");
        }
        if let Err(err) = validate_id(app_id) {
            bail!("invalid app ID: {err}");
        }
        let url = format!("https://catalog.fireflyzero.com/{path}.json");
        let resp = ureq::get(&url).call().context("send HTTP request")?;
        let mut body = resp.into_body().into_reader();
        let app: CatalogApp = serde_json::from_reader(&mut body).context("parse JSON")?;
        // TODO(@orsinium): the download link might be a download page,
        // not the actual ROM file.
        path = app.download;
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
    let mut body = resp.into_body().into_reader();
    std::io::copy(&mut body, &mut file).context("write response into a file")?;
    println!("⌛ installing...");
    Ok(out_path)
}

/// Read and parse app metadata from the app archive.
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

fn reset_launcher_cache(vfs_path: &Path) -> anyhow::Result<()> {
    let cache_path = vfs_path
        .join("data")
        .join("sys")
        .join("launcher")
        .join("etc")
        .join("metas");
    if cache_path.exists() {
        std::fs::remove_file(cache_path)?;
    }
    Ok(())
}

/// Verify SHA256 hash.
fn verify_hash(rom_path: &Path) -> anyhow::Result<()> {
    let hash_path = rom_path.join(HASH);
    let hash_expected: &[u8] = &fs::read(hash_path).context("read hash file")?;
    let hash_actual: &[u8] = &hash_dir(rom_path).context("calculate hash")?[..];
    if hash_actual != hash_expected {
        let exp = HEXLOWER.encode(hash_expected);
        let act = HEXLOWER.encode(hash_actual);
        bail!("expected: {exp}, got: {act}");
    }
    Ok(())
}

/// Create data dir and empty subdirs for the app.
pub(super) fn create_data_dir(meta: &Meta<'_>, vfs_path: &Path) -> anyhow::Result<()> {
    let data_path = vfs_path.join("data").join(meta.author_id).join(meta.app_id);
    let shots_path = data_path.join("shots");
    if !shots_path.exists() {
        fs::create_dir_all(&shots_path).context("create shots dir")?;
    }
    let etc_path = data_path.join("etc");
    if !etc_path.exists() {
        fs::create_dir_all(&etc_path).context("create etc dir")?;
    }
    Ok(())
}

/// Create or update app stats in the data dir based on the default stats file from ROM.
pub(super) fn write_stats(meta: &Meta<'_>, vfs_path: &Path) -> anyhow::Result<()> {
    let data_path = vfs_path.join("data").join(meta.author_id).join(meta.app_id);
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
    let today = chrono::Local::now().date_naive();
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let today = (
        today.year() as u16,
        today.month0() as u8,
        today.day0() as u8,
    );
    let default = if default_path.exists() {
        let raw = fs::read(default_path).context("read default stats file")?;
        firefly_types::Stats::decode(&raw)?
    } else {
        firefly_types::Stats {
            minutes: [0; 4],
            longest_play: [0; 4],
            launches: [0; 4],
            installed_on: today,
            updated_on: today,
            launched_on: (0, 0, 0),
            xp: 0,
            badges: Box::new([]),
            scores: Box::new([]),
        }
    };
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
    if !default_path.exists() {
        return Ok(());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::*;
    use crate::commands::*;
    use crate::test_helpers::*;

    #[test]
    fn test_build_export_import() {
        let vfs = make_tmp_vfs();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let args = BuildArgs {
            root: root.join("test_app"),
            ..Default::default()
        };
        cmd_build(vfs.clone(), &args).unwrap();

        let tmp_dir = make_tmp_dir();
        let archive_path = tmp_dir.join("test-app-export.zip");
        let args = ExportArgs {
            root: root.join("test_app"),
            id: Some("demo.cli-test".to_string()),
            output: Some(archive_path.clone()),
        };
        cmd_export(&vfs, &args).unwrap();

        let vfs2 = make_tmp_vfs();
        let path_str = archive_path.to_str().unwrap();
        let args = ImportArgs {
            path: path_str.to_string(),
        };
        cmd_import(&vfs2, &args).unwrap();

        dirs_eq(&vfs.join("roms"), &vfs2.join("roms"));
        dirs_eq(&vfs.join("data"), &vfs2.join("data"));
    }

    #[test]
    fn test_id_matches() {
        let meta = Meta {
            author_id: "sys",
            app_id: "launcher",

            app_name: "",
            author_name: "",
            launcher: true,
            sudo: true,
            version: 1,
        };
        assert!(id_matches("launcher", &meta));
        assert!(id_matches("sys.launcher", &meta));
        assert!(id_matches("sys.launcher.zip", &meta));
        assert!(id_matches("/tmp/sys.launcher.zip", &meta));
        let url = "https://github.com/firefly-zero/firefly-launcher/releases/latest/download/sys.launcher.zip";
        assert!(id_matches(url, &meta));

        assert!(!id_matches("lux.snek", &meta));
        assert!(!id_matches("snek", &meta));
    }
}
