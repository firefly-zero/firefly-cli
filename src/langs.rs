use crate::config::{Config, Lang};
use crate::wasm::strip_custom;
use anyhow::{bail, Context};
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
    }?;
    let bin_path = config.rom_path.join("bin");
    strip_custom(&bin_path)
}

fn detect_lang(root: &Path) -> anyhow::Result<Lang> {
    if root.join("go.mod").exists() {
        return Ok(Lang::Go);
    }
    if root.join("Cargo.toml").exists() {
        return Ok(Lang::Rust);
    }
    // Rust examples don't have Cargo.toml
    if root.join("main.rs").exists() {
        return Ok(Lang::Rust);
    }
    if root.join("build.zig").exists() {
        return Ok(Lang::Zig);
    }
    if root.join("build.zig.zon").exists() {
        return Ok(Lang::Zig);
    }
    if root.join("package.json").exists() {
        return Ok(Lang::TS);
    }
    bail!("failed to detect the programming language");
}

/// Build Go code using TinyGo.
fn build_go(config: &Config) -> anyhow::Result<()> {
    let target_path = find_tinygo_target(config)?;
    let target_path = path_to_utf8(&target_path)?;
    let out_path = config.rom_path.join("bin");
    let out_path = path_to_utf8(&out_path)?;
    let in_path = path_to_utf8(&config.root_path)?;
    let output = Command::new("tinygo")
        .args(["build", "-target", target_path, "-o", out_path, "."])
        .current_dir(in_path)
        .output()
        .context("run tinygo build")?;
    check_output(output)
}

/// Get the path to target.json in the project root or create a temporary one.
fn find_tinygo_target(config: &Config) -> anyhow::Result<PathBuf> {
    let target_path = config.root_path.join("target.json");
    if target_path.is_file() {
        return Ok(target_path);
    }
    let target_path = temp_dir().join("firefly-tinygo-target.json");
    let mut target_file = File::create(&target_path).context("create temporary file")?;
    let target_raw = include_bytes!("target.json");
    target_file
        .write_all(target_raw)
        .context("write temp file")?;
    Ok(target_path)
}

fn build_rust(config: &Config) -> anyhow::Result<()> {
    if config.root_path.join("Cargo.toml").exists() {
        build_rust_project(config)
    } else {
        build_rust_example(config)
    }
}

/// Build rust code example (must be a directory).
///
/// http://xion.io/post/code/rust-examples.html
fn build_rust_example(config: &Config) -> anyhow::Result<()> {
    let example_name = match config.root_path.file_name() {
        Some(dir_name) => dir_name,
        None => bail!("empty project path"),
    };
    let Some(example_name) = example_name.to_str() else {
        bail!("cannot convert project directory name to UTF-8")
    };
    let cargo_out_dir = temp_dir();
    let in_path = path_to_utf8(&config.root_path)?;
    let cmd_args = [
        "build",
        "--target",
        "wasm32-unknown-unknown",
        "--out-dir",
        path_to_utf8(&cargo_out_dir)?,
        "-Z",
        "unstable-options",
        "--example",
        example_name,
    ];
    let output = Command::new("cargo")
        .args(cmd_args)
        .current_dir(in_path)
        .output()
        .context("run cargo build")?;
    check_output(output)?;
    let cargo_out_path = cargo_out_dir.join(format!("{example_name}.wasm"));
    let out_path = config.rom_path.join("bin");
    std::fs::copy(cargo_out_path, out_path)?;
    Ok(())
}

fn build_rust_project(_config: &Config) -> anyhow::Result<()> {
    todo!()
}

fn build_zig(_config: &Config) -> anyhow::Result<()> {
    todo!()
}

fn build_ts(_config: &Config) -> anyhow::Result<()> {
    todo!()
}

/// Convert a file system path to UTF-8 if possible.
fn path_to_utf8(path: &Path) -> anyhow::Result<&str> {
    match path.to_str() {
        Some(path) => Ok(path),
        None => bail!("project root path cannot be converted to UTF-8"),
    }
}

fn check_output(output: Output) -> anyhow::Result<()> {
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        bail!("subprocess exited with status code {code}");
    }
    Ok(())
}
