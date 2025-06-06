use crate::args::BuildArgs;
use crate::audio::convert_wav;
use crate::commands::import::write_stats;
use crate::config::{Config, FileConfig};
use crate::crypto::hash_dir;
use crate::file_names::*;
use crate::fs::{collect_sizes, format_size};
use crate::images::convert_image;
use crate::langs::build_bin;
use crate::vfs::init_vfs;
use anyhow::{bail, Context};
use chrono::Datelike;
use crossterm::style::Stylize;
use data_encoding::HEXLOWER;
use firefly_types::Encode;
use rand::Rng;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::signature::hazmat::PrehashSigner;
use rsa::signature::SignatureEncoding;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

static TIPS: &[&str] = &[
    "keep an eye on the binary size: bigger binary often means slower code",
    "if the app hits `unreachable`, use `log_debug` to find out where",
    "you can use `build_args` option in `firefly.toml` to customize the build command",
    "if your game has multiple levels/scenes, use a separate sprite file for each",
    "prefer using 32 bit float over 64 bit float",
    "using shapes instead of sprites might save memory and improve performance",
    "if you specify `url` for a file in `firefly.toml`, provide `sha256` as well",
    "if your app is open-source, don't forget to add a LICENSE file",
    "the desktop emulator has some useful CLI flags, like --fullscreen",
    "pick a short name for the app so that it looks good in the launcher",
    "setting a custom color palette may give your app a distinct memorable style",
    "the desktop emulator supports gamepads",
    "if building is slow, try skipping optimizations: `--no-opt --no-strip`",
    "if your compiler allows it, pick a small allocator and garbage collector",
    "create `.firefly` dir in the project root to store VFS in there",
    "you can customize TinyGo build with a custom target.json in the project root",
    "make sure to test your game with multiplayer",
    "images using 4 or less colors are twice smaller",
    "when debugging an app, call `set_seed` in `boot` to make the randomness predictable",
    // covering CLI subcommands
    "you can use `wasm2wat` and `firefly_cli inspect` to inspect the app binary",
    "use `firefly_cli export` to share the app with your friends",
    "backup your private key (but keep it secret!): firefly_cli key priv",
    "use `firefly_cli monitor` to see RAM and CPU consumption of a running app",
    // our internal resources
    "when the app is ready, consider adding it to https://catalog.fireflyzero.com",
    "you can download fonts from https://fonts.fireflyzero.com/",
    "follow us on Mastodon for updates: https://fosstodon.org/@fireflyzero",
    // We're not paid for any of these links, it's just resources we love.
    "good free sprite editor: https://apps.lospec.com/pixel-editor/",
    "good collection of free game assets: https://opengameart.org/",
    "https://youtu.be/dQw4w9WgXcQ",
];

pub fn cmd_build(vfs: PathBuf, args: &BuildArgs) -> anyhow::Result<()> {
    init_vfs(&vfs).context("init vfs")?;
    let config = Config::load(vfs, &args.root).context("load project config")?;
    if config.author_id == "joearms" {
        println!("⚠️  author_id in firefly.tom has the default value.");
        println!("  Please, change it before sharing the app with the world.");
    }
    if !args.no_tip {
        show_tip();
    }
    let old_sizes = collect_sizes(&config.rom_path);
    let meta = write_meta(&config).context("write metadata file")?;
    build_bin(&config, args).context("build binary")?;
    remove_old_files(&config.rom_path).context("remove old files")?;
    if let Some(files) = &config.files {
        for (name, file_config) in files {
            convert_file(name, &config, file_config)
                .with_context(|| format!("convert \"{name}\""))?;
        }
    }
    write_badges(&config).context("write badges")?;
    write_boards(&config).context("write boards")?;
    write_installed(&config).context("write app-name")?;
    create_rom_stats(&config).context("create default stats file")?;
    write_stats(&meta, &config.vfs_path).context("write stats")?;
    write_key(&config).context("write key")?;
    write_hash(&config.rom_path).context("write hash")?;
    write_sig(&config).context("sign ROM")?;
    let new_sizes = collect_sizes(&config.rom_path);
    check_sizes(&new_sizes)?;
    print_sizes(&old_sizes, &new_sizes);
    println!("\n✅ installed: {}.{}", config.author_id, config.app_id);
    Ok(())
}

/// Serialize and write the ROM meta information.
fn write_meta(config: &Config) -> anyhow::Result<firefly_types::Meta<'_>> {
    use firefly_types::{validate_id, validate_name, Meta};
    if let Err(err) = validate_id(&config.app_id) {
        bail!("validate app_id: {err}");
    }
    if let Err(err) = validate_id(&config.author_id) {
        bail!("validate author_id: {err}");
    }
    if let Err(err) = validate_name(&config.app_name) {
        bail!("validate app_name: {err}");
    }
    if let Err(err) = validate_name(&config.author_name) {
        bail!("validate author_name: {err}");
    }
    let meta = Meta {
        app_id: &config.app_id,
        app_name: &config.app_name,
        author_id: &config.author_id,
        author_name: &config.author_name,
        launcher: config.launcher,
        sudo: config.sudo,
        version: config.version.unwrap_or(0),
    };
    let encoded = meta.encode_vec().context("serialize")?;
    fs::create_dir_all(&config.rom_path)?;
    let output_path = config.rom_path.join(META);
    fs::write(output_path, encoded).context("write file")?;
    Ok(meta)
}

/// Write the latest installed app name into internal DB.
fn write_installed(config: &Config) -> anyhow::Result<()> {
    let short_meta = firefly_types::ShortMeta {
        app_id: &config.app_id,
        author_id: &config.author_id,
    };
    let encoded = short_meta.encode_vec().context("serialize")?;
    let output_path = config.vfs_path.join("sys").join("new-app");
    fs::write(output_path, &encoded).context("write new-app file")?;
    if config.launcher {
        let output_path = config.vfs_path.join("sys").join("launcher");
        fs::write(output_path, encoded).context("write launcher file")?;
    }
    Ok(())
}

/// Remove all files except `_meta` and `_bin`.
///
/// The function is called after writing meta and binary but before writing
/// anything else. We can't call it earlier because if building binary fails,
/// files are already removed, and on the next run the file size diff
/// will show like these files are created for the first time.
///
/// We need to remove files to avoid old files surviving app update.
fn remove_old_files(root: &Path) -> anyhow::Result<()> {
    let dir = fs::read_dir(root)?;
    for entry in dir {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else if meta.is_file() {
            let file_name = entry.file_name().into_string().unwrap();
            if file_name == BIN || file_name == META {
                continue;
            }
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

/// Get a file from config, convert it if needed, and write into the ROM.
fn convert_file(name: &str, config: &Config, file_config: &FileConfig) -> anyhow::Result<()> {
    if name == SIG || name == META || name == HASH || name == KEY {
        bail!("ROM file name \"{name}\" is reserved");
    }
    let output_path = config.rom_path.join(name);
    // The input path is defined in the config
    // and should be resolved relative to the project root.
    let input_path = &config.root_path.join(&file_config.path);
    download_file(input_path, file_config).context("download file")?;
    if file_config.copy {
        fs::copy(input_path, &output_path)?;
        return Ok(());
    }
    let Some(extension) = input_path.extension() else {
        let file_name = input_path.to_str().unwrap().to_string();
        bail!("cannot detect extension for {file_name}");
    };
    let Some(extension) = extension.to_str() else {
        bail!("cannot convert file extension to string");
    };
    match extension {
        "png" => {
            convert_image(input_path, &output_path)?;
        }
        "wav" => {
            convert_wav(input_path, &output_path)?;
        }
        // firefly formats for fonts and images
        "fff" | "ffi" | "ffz" => {
            fs::copy(input_path, &output_path)?;
        }
        _ => bail!("unknown file extension: {extension}"),
    }
    Ok(())
}

/// If file doesn't exist, donload it from `url` and validate `sha256`.
fn download_file(input_path: &Path, file_config: &FileConfig) -> anyhow::Result<()> {
    if input_path.exists() {
        return Ok(());
    }
    let Some(url) = &file_config.url else {
        bail!("file does not exist and no url specified");
    };
    let resp = ureq::get(url).call().context("send request")?;
    let mut bytes: Vec<u8> = vec![];
    resp.into_reader()
        .read_to_end(&mut bytes)
        .context("read response")?;
    if let Some(expected_hash) = &file_config.sha256 {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual_hash = HEXLOWER.encode(&hasher.finalize());
        if actual_hash != *expected_hash {
            bail!("sha256 hash mismatch: {actual_hash} != {expected_hash}");
        }
    }
    fs::write(input_path, bytes).context("write file")?;
    Ok(())
}

fn write_badges(config: &Config) -> anyhow::Result<()> {
    let configs = config.badges_vec()?;
    if configs.is_empty() {
        return Ok(());
    }
    // collect and convert badges
    let mut badges: Vec<firefly_types::Badge<'_>> = Vec::new();
    for (badge, id) in configs.iter().zip(1u16..) {
        if badge.steps == Some(0) {
            bail!("validate badge #{id}: steps must not be zero");
        }
        let badge = firefly_types::Badge {
            position: badge.position.unwrap_or(id),
            xp: badge.xp.unwrap_or(1),
            hidden: badge.hidden,
            name: &badge.name,
            descr: &badge.descr,
        };
        if let Err(err) = badge.validate() {
            bail!("validate badge #{id}: {err}");
        }
        badges.push(badge);
    }

    // write badges to the file
    let badges = firefly_types::Badges::new(Cow::Owned(badges));
    let encoded = badges.encode_vec().context("serialize")?;
    let output_path = config.rom_path.join(BADGES);
    fs::write(output_path, encoded).context("write file")?;
    Ok(())
}

/// Write boards (aka scoreboards or leaderboards) info into the ROM.
fn write_boards(config: &Config) -> anyhow::Result<()> {
    // some basic validations
    let Some(configs) = &config.boards else {
        return Ok(());
    };
    if configs.is_empty() {
        return Ok(());
    }
    if configs.get("0").is_some() {
        bail!("board IDs must start at 1")
    }
    let len = configs.len();
    if len > 20 {
        bail!("too many boards")
    }
    let len = u16::try_from(len).unwrap();

    // collect and convert boards
    let mut boards: Vec<firefly_types::Board<'_>> = Vec::new();
    for id in 1u16..=len {
        let Some(board) = configs.get(&id.to_string()) else {
            bail!("board IDs must be consequentive but ID {id} is missed");
        };
        let board = firefly_types::Board {
            position: board.position.unwrap_or(id),
            min: board.max.unwrap_or(i16::MIN),
            max: board.min.unwrap_or(i16::MAX),
            time: board.time,
            decimals: board.decimals,
            name: &board.name,
        };
        if let Err(err) = board.validate() {
            bail!("validate board #{id}: {err}");
        }
        boards.push(board);
    }

    // write boards to the file
    let boards = firefly_types::Boards::new(Cow::Owned(boards));
    let encoded = boards.encode_vec().context("serialize")?;
    let output_path = config.rom_path.join(BOARDS);
    fs::write(output_path, encoded).context("write file")?;
    Ok(())
}

/// Create default app stats.
fn create_rom_stats(config: &Config) -> anyhow::Result<()> {
    let path = config.rom_path.join(STATS);
    let today = chrono::Local::now().date_naive();
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let today = (
        today.year() as u16,
        today.month0() as u8,
        today.day0() as u8,
    );
    let mut badges = Vec::new();
    let badges_config = config.badges_vec()?;
    for badge in badges_config {
        badges.push(firefly_types::BadgeProgress {
            new: false,
            done: 0,
            goal: badge.steps.unwrap_or(1),
        });
    }
    let mut scores = Vec::new();
    if let Some(boards_config) = &config.boards {
        for _ in 0..boards_config.len() {
            let fs = firefly_types::FriendScore { index: 0, score: 0 };
            let score = firefly_types::BoardScores {
                me: Box::new([0i16; 8]),
                friends: Box::new([fs; 8]),
            };
            scores.push(score);
        }
    }
    let stats = firefly_types::Stats {
        minutes: [0; 4],
        longest_play: [0; 4],
        launches: [0; 4],
        installed_on: today,
        updated_on: today,
        launched_on: (0, 0, 0),
        xp: 0,
        badges: badges.into_boxed_slice(),
        scores: scores.into_boxed_slice(),
    };
    let encoded = stats.encode_vec().context("serialize")?;
    fs::write(path, encoded).context("write file")?;
    Ok(())
}

/// Copy the public key for the author into the ROM.
fn write_key(config: &Config) -> anyhow::Result<()> {
    let sys_path = config.vfs_path.join("sys");
    let author_id = &config.author_id;
    let pub_path = sys_path.join("pub").join(author_id);
    if !pub_path.exists() {
        // Don't show error here just yet.
        // If the key is missed, the error will be reported later by write_sig.
        return Ok(());
    }
    let key_path = config.rom_path.join(KEY);
    fs::copy(pub_path, key_path).context("copy public key")?;
    Ok(())
}

/// Generate SHA256 hash for all the ROM files.
fn write_hash(rom_path: &Path) -> anyhow::Result<()> {
    let hash = hash_dir(rom_path)?;
    let hash_path = rom_path.join(HASH);
    let mut hash_file = fs::File::create(hash_path).context("create file")?;
    hash_file.write_all(&hash[..]).context("write file")
}

/// Sign the ROM hash.
fn write_sig(config: &Config) -> anyhow::Result<()> {
    let sys_path = config.vfs_path.join("sys");
    let author_id = &config.author_id;
    let pub_path = sys_path.join("pub").join(author_id);
    if !pub_path.exists() {
        println!("⚠️  no key found for {author_id}, cannot sign ROM");
        return Ok(());
    }
    let priv_path = sys_path.join("priv").join(author_id);
    if !priv_path.exists() {
        println!("⚠️  there is only public key for {author_id}, cannot sign ROM");
        return Ok(());
    }

    let key_bytes = fs::read(priv_path).context("read private key")?;
    let private_key = RsaPrivateKey::from_pkcs1_der(&key_bytes).context("parse key")?;
    let signing_key = SigningKey::<Sha256>::new(private_key);

    let hash_path = config.rom_path.join(HASH);
    let hash_bytes = fs::read(hash_path).context("read hash")?;

    let sig = signing_key.sign_prehash(&hash_bytes).context("sign hash")?;
    let sig_bytes = sig.to_bytes();
    let sig_path = config.rom_path.join(SIG);
    fs::write(sig_path, sig_bytes).context("write signature to file")?;

    Ok(())
}

/// Check that there are now big or empty files in the ROM.
fn check_sizes(sizes: &HashMap<OsString, u64>) -> anyhow::Result<()> {
    const MB: u64 = 1024 * 1024;
    for (name, size) in sizes {
        if *size == 0 {
            let name = name.to_str().unwrap_or("???");
            bail!("the file {name} is empty");
        }
        if *size > 10 * MB {
            let name = name.to_str().unwrap_or("???");
            bail!("the file {name} is too big");
        }
    }
    Ok(())
}

/// Show the table of file sizes and how they changed from the previous build.
fn print_sizes(old_sizes: &HashMap<OsString, u64>, new_sizes: &HashMap<OsString, u64>) {
    let mut pairs: Vec<_> = new_sizes.iter().collect();
    pairs.sort();
    for (name, new_size) in pairs {
        let old_size = old_sizes.get(name).unwrap_or(&0);
        let Some(name) = name.to_str() else {
            continue;
        };

        // If the size changed, show the diff size
        let suffix = if old_size == new_size {
            String::new()
        } else {
            #[expect(clippy::cast_possible_wrap)]
            let diff = *new_size as i64 - *old_size as i64;
            let suffix = format!(" ({diff:+})");
            if *old_size == 0 {
                suffix
            } else if diff > 0 {
                format!("{}", suffix.red())
            } else {
                format!("{}", suffix.green())
            }
        };

        let new_size = format_size(*new_size);
        println!("{name:16} {new_size}{suffix}");
    }
}

fn show_tip() {
    let mut rng = rand::thread_rng();
    let i = rng.gen_range(0..TIPS.len());
    println!("💡 tip: {}.", TIPS[i]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn test_build() {
        let vfs = make_tmp_vfs();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let args = BuildArgs {
            root: root.join("test_app"),
            ..Default::default()
        };
        cmd_build(vfs, &args).unwrap();
    }
}
