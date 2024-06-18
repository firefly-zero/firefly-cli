use crate::args::BuildArgs;
use crate::config::{Config, Lang};
use crate::file_names::BIN;
use crate::wasm::{optimize, strip_custom};
use anyhow::{bail, Context};
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn build_bin(config: &Config, args: &BuildArgs) -> anyhow::Result<()> {
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
    let bin_path = config.rom_path.join(BIN);
    if !args.no_strip {
        strip_custom(&bin_path)?;
    }
    if !args.no_opt {
        optimize(&bin_path).context("optimize wasm binary")?;
    }
    Ok(())
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

/// Build Go code using [TinyGo].
///
/// [TinyGo]: https://tinygo.org/
fn build_go(config: &Config) -> anyhow::Result<()> {
    let target_path = find_tinygo_target(config)?;
    let target_path = path_to_utf8(&target_path)?;
    let out_path = config.rom_path.join(BIN);
    let out_path = path_to_utf8(&out_path)?;
    let in_path = path_to_utf8(&config.root_path)?;
    let mut cmd_args = vec!["build", "-target", target_path, "-o", out_path, "."];
    if let Some(additional_args) = &config.compile_args {
        for arg in additional_args {
            cmd_args.push(arg.as_str());
        }
    }
    let output = Command::new("tinygo")
        .args(cmd_args)
        .current_dir(in_path)
        .output()
        .context("run tinygo build")?;
    check_output(&output)
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
        build_rust_inner(config, false)
    } else {
        // Build rust code example (must be a directory).
        //
        // See [Add examples to your Rust libraries][1] to learn more about
        // how directory-based examples in Rust work.
        //
        // [1]: http://xion.io/post/code/rust-examples.html
        build_rust_inner(config, true)
    }
}

fn build_rust_inner(config: &Config, example: bool) -> anyhow::Result<()> {
    let Some(example_name) = config.root_path.file_name() else {
        bail!("empty project path");
    };
    let Some(example_name) = example_name.to_str() else {
        bail!("cannot convert project directory name to UTF-8")
    };
    let in_path = path_to_utf8(&config.root_path)?;
    let mut cmd_args = vec!["build", "--target", "wasm32-unknown-unknown", "--release"];
    if example {
        cmd_args.push("--example");
        cmd_args.push(example_name);
    }
    if let Some(additional_args) = &config.compile_args {
        for arg in additional_args {
            cmd_args.push(arg.as_str());
        }
    }
    let output = Command::new("cargo")
        .args(cmd_args)
        .current_dir(in_path)
        .output()
        .context("run cargo build")?;
    check_output(&output)?;
    let cargo_out_path = find_rust_result(&config.root_path)?;
    let out_path = config.rom_path.join(BIN);
    std::fs::copy(cargo_out_path, out_path)?;
    Ok(())
}

/// Locate the wasm binary produced by `cargo build`.
fn find_rust_result(root: &Path) -> anyhow::Result<PathBuf> {
    let target_dir = find_rust_target_dir(root)?;
    let release_dir = target_dir.join("wasm32-unknown-unknown").join("release");
    let Some(project_name) = root.file_name() else {
        bail!("cannot get project root directory name");
    };

    let path = release_dir.join(project_name).with_extension("wasm");
    if path.is_file() {
        return Ok(path);
    }
    let path = release_dir
        .join("examples")
        .join(project_name)
        .with_extension("wasm");
    if path.is_file() {
        return Ok(path);
    }
    bail!("cannot find wasm binary")
}

/// Locate the "target" directory.
///
/// If building an example or a crate in a workspace,
/// the "target" directory might be located not in the given project root
/// but in one of the parent directorries. So, this function goes up
/// the file tree until it finds the target dir.
fn find_rust_target_dir(root: &Path) -> anyhow::Result<PathBuf> {
    let root = root
        .canonicalize()
        .context("canonicalize project root path")?;
    let mut maybe_path = Some(root.as_path());
    while let Some(path) = maybe_path {
        let target_path = path.join("target");
        if target_path.exists() {
            return Ok(target_path);
        }
        maybe_path = path.parent();
    }
    bail!("cannot find Rust's \"target\" output directory")
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

fn check_output(output: &Output) -> anyhow::Result<()> {
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        bail!("subprocess exited with status code {code}");
    }
    Ok(())
}
