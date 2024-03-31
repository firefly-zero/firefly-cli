use crate::args::{BuildArgs, Lang};
use crate::error::CLIError;
use std::path::Path;

pub(crate) fn cmd_build(args: &BuildArgs) -> Result<(), CLIError> {
    let root = Path::new(&args.input);
    let lang = detect_lang(root)?;
    match lang {
        Lang::Go => build_go(args),
        Lang::Rust => build_rust(args),
        Lang::Zig => build_zig(args),
        Lang::TS => build_ts(args),
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

fn build_go(_args: &BuildArgs) -> Result<(), CLIError> {
    todo!()
}

fn build_rust(_args: &BuildArgs) -> Result<(), CLIError> {
    todo!()
}

fn build_zig(_args: &BuildArgs) -> Result<(), CLIError> {
    todo!()
}

fn build_ts(_args: &BuildArgs) -> Result<(), CLIError> {
    todo!()
}
