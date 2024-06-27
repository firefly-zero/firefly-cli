use crate::args::BuildArgs;
use crate::config::{Config, FileConfig};
use crate::crypto::hash_dir;
use crate::file_names::{HASH, KEY, META, SIG};
use crate::images::convert_image;
use crate::langs::build_bin;
use crate::vfs::init_vfs;
use anyhow::{bail, Context};
use colored::Colorize;
use data_encoding::HEXLOWER;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::signature::hazmat::PrehashSigner;
use rsa::signature::SignatureEncoding;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn cmd_build(args: &BuildArgs) -> anyhow::Result<()> {
    init_vfs().context("init vfs")?;
    let config = Config::load(&args.root).context("load project config")?;
    let old_sizes = collect_sizes(&config.rom_path);
    _ = fs::remove_dir_all(&config.rom_path);
    write_meta(&config).context("write metadata file")?;
    build_bin(&config, args).context("build binary")?;
    if let Some(files) = &config.files {
        for (name, file_config) in files {
            convert_file(name, &config, file_config).context("convert file")?;
        }
    }
    write_installed(&config).context("write app-name")?;
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
fn write_meta(config: &Config) -> anyhow::Result<()> {
    use firefly_meta::{validate_id, validate_name, Meta};
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
        app_id:      &config.app_id,
        app_name:    &config.app_name,
        author_id:   &config.author_id,
        author_name: &config.author_name,
        launcher:    config.launcher,
        sudo:        config.sudo,
        version:     config.version.unwrap_or(0),
    };
    let mut buf = vec![0; meta.size()];
    let encoded = meta.encode(&mut buf).context("serialize")?;
    fs::create_dir_all(&config.rom_path)?;
    let output_path = config.rom_path.join(META);
    fs::write(output_path, encoded).context("write file")?;
    Ok(())
}

/// Write the latest installed app name into internal DB.
fn write_installed(config: &Config) -> anyhow::Result<()> {
    let short_meta = firefly_meta::ShortMeta {
        app_id:    &config.app_id,
        author_id: &config.author_id,
    };
    let mut buf = vec![0; short_meta.size()];
    let encoded = short_meta.encode(&mut buf).context("serialize")?;
    let output_path = config.vfs_path.join("sys").join("new-app");
    fs::write(output_path, &encoded).context("write new-app file")?;
    if config.launcher {
        let output_path = config.vfs_path.join("sys").join("launcher");
        fs::write(output_path, &encoded).context("write launcher file")?;
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
    std::fs::write(input_path, bytes).context("write file")?;
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
    hash_file.write_all(&hash).context("write file")
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

/// Get size in bytes for every file in the ROM directory.
fn collect_sizes(root: &Path) -> HashMap<OsString, u64> {
    let mut sizes = HashMap::new();
    let Ok(entries) = fs::read_dir(root) else {
        return sizes;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(meta) = entry.metadata() else { continue };
        sizes.insert(entry.file_name(), meta.len());
    }
    sizes
}

/// Check that there are now big or empty files in the ROM.
fn check_sizes(sizes: &HashMap<OsString, u64>) -> anyhow::Result<()> {
    const MB: u64 = 1024 * 1024;
    for (name, size) in sizes {
        if *size == 0 {
            let name = name.to_str().unwrap_or("___");
            bail!("the file {name} is empty");
        }
        if *size > 10 * MB {
            let name = name.to_str().unwrap_or("___");
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
            #[allow(clippy::cast_possible_wrap)]
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

        // convert big file size into Kb or Mb.
        let new_size = if *new_size > 1024 * 1024 {
            let new_size = new_size / 1024 / 1024;
            format!("{new_size:>7} {}", "Mb".purple())
        } else if *new_size > 1024 {
            let new_size = new_size / 1024;
            format!("{new_size:>7} {}", "Kb".blue())
        } else {
            format!("{new_size:>10}")
        };

        println!("{name:16} {new_size}{suffix}");
    }
}
