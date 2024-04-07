use crate::config::{Config, Lang};
use crate::error::CLIError;
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub(crate) fn build_bin(config: &Config) -> Result<(), CLIError> {
    let root = Path::new(&config.root);
    let lang: Lang = match &config.lang {
        Some(lang) => lang.clone(),
        None => detect_lang(root)?,
    };
    match lang {
        Lang::Go => build_go(config),
        Lang::Rust => build_rust(config),
        Lang::Zig => build_zig(config),
        Lang::TS => build_ts(config),
    }
}

fn detect_lang(root: &Path) -> Result<Lang, CLIError> {
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
    Err(CLIError::LangNotDetected)
}

fn build_go(config: &Config) -> Result<(), CLIError> {
    let target_path = temp_dir().join("firefly-tinygo-target.json");
    let mut target_file = match File::create(&target_path) {
        Ok(target_file) => target_file,
        Err(err) => CLIError::wrap("create temp file", err.into())?,
    };
    if let Err(err) = target_file.write_all(include_bytes!("target.json")) {
        CLIError::wrap("write temp file", err.into())?;
    };
    let target_path_str: &str = target_path.to_str().unwrap();
    let out_path = config.rom_path().join("cart.wasm");
    let out_path = std::fs::canonicalize(out_path)?;
    let out_path = out_path.to_str().unwrap();
    let in_path = config.root.to_str().unwrap();
    let output = Command::new("tinygo")
        .args(["build", "-target", target_path_str, "-o", out_path, "."])
        .current_dir(in_path)
        .output()?;
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        return Err(CLIError::Subprocess(output.status.code().unwrap_or(1)));
    }
    Ok(())
}

fn build_rust(_config: &Config) -> Result<(), CLIError> {
    todo!()
}

fn build_zig(_config: &Config) -> Result<(), CLIError> {
    todo!()
}

fn build_ts(_config: &Config) -> Result<(), CLIError> {
    todo!()
}
