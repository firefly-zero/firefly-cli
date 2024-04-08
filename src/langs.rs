use crate::config::{Config, Lang};
use anyhow::{bail, Context};
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub(crate) fn build_bin(config: &Config) -> anyhow::Result<()> {
    let lang: Lang = match &config.lang {
        Some(lang) => lang.clone(),
        None => detect_lang(&config.root_path)?,
    };
    match lang {
        Lang::Go => build_go(config),
        Lang::Rust => build_rust(config),
        Lang::Zig => build_zig(config),
        Lang::TS => build_ts(config),
    }
}

fn detect_lang(root: &Path) -> anyhow::Result<Lang> {
    if root.join("go.mod").exists() {
        return Ok(Lang::Go);
    }
    if root.join("Cargo.toml").exists() {
        return Ok(Lang::Rust);
    }
    if root.join("build.zig").exists() {
        return Ok(Lang::Zig);
    }
    if root.join("package.json").exists() {
        return Ok(Lang::TS);
    }
    bail!("failed to detect the programming language");
}

fn build_go(config: &Config) -> anyhow::Result<()> {
    let target_path = temp_dir().join("firefly-tinygo-target.json");
    let mut target_file = File::create(&target_path).context("create temporary file")?;
    let target_raw = include_bytes!("target.json");
    target_file
        .write_all(target_raw)
        .context("write temp file")?;
    let target_path_str: &str = target_path.to_str().unwrap();
    let current_dir = std::env::current_dir().context("get current directory")?;
    let rom_path = current_dir.join(&config.rom_path);
    let out_path = rom_path.join("cart.wasm");
    let out_path = out_path.to_str().unwrap();
    let in_path = config.root_path.to_str().unwrap();
    let output = Command::new("tinygo")
        .args(["build", "-target", target_path_str, "-o", out_path, "."])
        .current_dir(in_path)
        .output()?;
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        bail!("subprocess exited with status code {code}");
    }
    Ok(())
}

fn build_rust(_config: &Config) -> anyhow::Result<()> {
    todo!()
}

fn build_zig(_config: &Config) -> anyhow::Result<()> {
    todo!()
}

fn build_ts(_config: &Config) -> anyhow::Result<()> {
    todo!()
}
