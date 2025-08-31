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
    // Don't build the binary if it will be copied directly in "files".
    if let Some(files) = &config.files {
        if files.contains_key(BIN) {
            return Ok(());
        }
    }
    let lang: Lang = match &config.lang {
        Some(lang) => lang.clone(),
        None => detect_lang(&config.root_path)?,
    };
    match lang {
        Lang::Go => build_go(config),
        Lang::Rust => build_rust(config),
        Lang::Zig => build_zig(config),
        Lang::TS => build_ts(config),
        Lang::C => build_c(config),
        Lang::Cpp => build_cpp(config),
        Lang::Python => build_python(config),
    }?;
    let bin_path = config.rom_path.join(BIN);
    if !bin_path.is_file() {
        bail!("the build command haven't produced a binary file");
    }
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
    if root.join("pyproject.toml").exists() {
        return Ok(Lang::Python);
    }
    if root.join("main.c").exists() {
        return Ok(Lang::C);
    }
    if root.join("main.cpp").exists() {
        return Ok(Lang::Cpp);
    }
    if root.join("src").join("main.c").exists() {
        return Ok(Lang::C);
    }
    if root.join("src").join("main.cpp").exists() {
        return Ok(Lang::Cpp);
    }
    bail!("failed to detect the programming language");
}

/// Build Go code using [TinyGo].
///
/// [TinyGo]: https://tinygo.org/
fn build_go(config: &Config) -> anyhow::Result<()> {
    check_installed("Go", "tinygo", "version")?;
    let target_path = find_tinygo_target(config)?;
    let target_path = path_to_utf8(&target_path)?;
    let out_path = config.rom_path.join(BIN);
    let out_path = path_to_utf8(&out_path)?;
    let in_path = path_to_utf8(&config.root_path)?;
    let mut cmd_args = vec![
        "build",
        "-target",
        target_path,
        "-o",
        out_path,
        "-buildmode",
        "c-shared",
        ".",
    ];
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

/// Build Rust project.
fn build_rust(config: &Config) -> anyhow::Result<()> {
    check_installed("Rust", "cargo", "version")?;
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
    let in_path = path_to_utf8(&config.root_path)?;
    let mut cmd_args = vec![
        "+nightly",
        "build",
        "-Zbuild-std",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
    ];
    if example {
        let Some(example_name) = config.root_path.file_name() else {
            bail!("empty project path");
        };
        let Some(example_name) = example_name.to_str() else {
            bail!("cannot convert project directory name to UTF-8")
        };
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
    if let Some(path) = find_wasm_binary(&release_dir)? {
        return Ok(path);
    }
    let examples_dir = release_dir.join("examples");
    if let Some(path) = find_wasm_binary(&examples_dir)? {
        return Ok(path);
    }
    bail!("cannot find wasm binary")
}

fn find_wasm_binary(root: &Path) -> anyhow::Result<Option<PathBuf>> {
    let entries = std::fs::read_dir(root)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        let Some(ext) = ext.to_str() else {
            continue;
        };
        if ext == "wasm" {
            return Ok(Some(path));
        }
    }
    Ok(None)
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

/// Build C project using wasi-sdk.
fn build_c(config: &Config) -> anyhow::Result<()> {
    build_cpp_inner(config, "clang", "main.c")
}

/// Build C++ project using wasi-sdk.
fn build_cpp(config: &Config) -> anyhow::Result<()> {
    build_cpp_inner(config, "clang++", "main.cpp")
}

/// Build C/C++ project using wasi-sdk.
fn build_cpp_inner(config: &Config, bin_name: &str, fname: &str) -> anyhow::Result<()> {
    let wasi_sdk = find_wasi_sdk()?;
    let mut in_path = &config.root_path.join(fname);
    let in_path_src = &config.root_path.join("src").join(fname);
    if !in_path.exists() {
        in_path = in_path_src;
        if !in_path.exists() {
            bail!("file {fname} not found");
        }
    }
    let out_path = config.rom_path.join(BIN);
    let wasi_sysroot = wasi_sdk.join("share").join("wasi-sysroot");
    let mut cmd_args = vec![
        "--sysroot",
        path_to_utf8(&wasi_sysroot)?,
        "-o",
        path_to_utf8(&out_path)?,
        "-mexec-model=reactor",
        "-Wl,--stack-first,--no-entry,--strip-all,--gc-sections,--lto-O3",
        "-Oz",
        path_to_utf8(in_path)?,
    ];
    if let Some(additional_args) = &config.compile_args {
        for arg in additional_args {
            cmd_args.push(arg.as_str());
        }
    } else {
        cmd_args.push("-Wl,-zstack-size=14752,--initial-memory=65536,--max-memory=65536");
    }
    let clang_path = wasi_sdk.join("bin").join(bin_name);
    let output = Command::new(path_to_utf8(&clang_path)?)
        .args(cmd_args)
        .current_dir(&config.root_path)
        .output()
        .context("run clang++")?;
    check_output(&output)?;
    Ok(())
}

/// find the wasi-sdk project root.
fn find_wasi_sdk() -> anyhow::Result<PathBuf> {
    if let Ok(path) = std::env::var("WASI_SDK_PATH") {
        let path = PathBuf::from(path);
        if !path.exists() {
            bail!("the path specified in $WASI_SDK_PATH does not exist");
        }
        if !path.is_dir() {
            bail!("the path specified in $WASI_SDK_PATH is not a directory");
        }
        return Ok(path);
    }
    let path = PathBuf::from("/opt/wasi-sdk");
    if !path.exists() {
        bail!("/opt/wasi-sdk does not exist");
    }
    if !path.is_dir() {
        bail!("/opt/wasi-sdk is not a directory");
    }
    Ok(path)
}

fn build_zig(config: &Config) -> anyhow::Result<()> {
    check_installed("Zig", "zig", "version")?;
    let mut cmd_args = vec!["build"];
    if let Some(additional_args) = &config.compile_args {
        for arg in additional_args {
            cmd_args.push(arg.as_str());
        }
    }
    let output = Command::new("zig")
        .args(cmd_args)
        .current_dir(&config.root_path)
        .output()
        .context("run zig build")?;
    check_output(&output)?;

    let from_dir = config.root_path.join("zig-out").join("bin");
    let from_path = find_wasm(&from_dir)?;
    let out_path = config.rom_path.join(BIN);
    std::fs::copy(&from_path, out_path).context("copy wasm binary")?;
    std::fs::remove_file(from_path).context("remove wasm file")?;
    Ok(())
}

/// Find a wasm binary in the given directory.
fn find_wasm(from_dir: &Path) -> anyhow::Result<PathBuf> {
    let from_dir = std::fs::read_dir(from_dir)?;
    let mut result = None;
    for file_path in from_dir {
        let file_path = file_path?;
        let file_path = file_path.path();
        if let Some(ext) = file_path.extension() {
            if ext == "wasm" {
                if result.is_some() {
                    bail!("found more than one wasm binary");
                }
                result = Some(file_path);
            }
        }
    }
    match result {
        Some(result) => Ok(result),
        None => bail!("cannot find wasm binary"),
    }
}

fn build_ts(_config: &Config) -> anyhow::Result<()> {
    todo!("TypeScript is not supported yet")
}

fn build_python(_config: &Config) -> anyhow::Result<()> {
    todo!("Python is not supported yet")
}

/// Convert a file system path to UTF-8 if possible.
pub fn path_to_utf8(path: &Path) -> anyhow::Result<&str> {
    match path.to_str() {
        Some(path) => Ok(path),
        None => bail!("project root path cannot be converted to UTF-8"),
    }
}

pub fn check_output(output: &Output) -> anyhow::Result<()> {
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        bail!("subprocess exited with status code {code}");
    }
    Ok(())
}

/// Run the given binary with the given arg and return an error if it is not installed.
pub fn check_installed(lang: &str, bin: &str, arg: &str) -> anyhow::Result<()> {
    use std::fmt::Write;

    let output = Command::new(bin).args([arg]).output();
    if let Ok(output) = output {
        if output.status.success() {
            return Ok(());
        }
    }
    let mut msg =
        format!("You're trying to build a {lang} app but you don't have {bin} installed.\n");
    _ = writeln!(msg, "Please, follow the getting started guide for {lang}:");
    _ = write!(msg, "  https://docs.fireflyzero.com/dev/getting-started/");
    bail!(msg);
}
