use std::{
    io::Write,
    path::Path,
    process::{Command, Output},
};

use crate::{args::NewArgs, config::Lang};
use anyhow::{bail, Context, Result};

const CONFIG: &str = r#"
author_id = "joearms"
app_id = "hello-world"
author_name = "Joe Armstrong"
app_name = "Hello World"
"#;

pub fn cmd_new(args: &NewArgs) -> Result<()> {
    if args.path.exists() {
        bail!("the directory already exists");
    };
    let lang = parse_lang(&args.lang)?;
    match lang {
        Lang::Go => todo!(),
        Lang::Rust => new_rust(&args.path).context("new rust project")?,
        Lang::Zig => todo!(),
        Lang::TS => todo!(),
        Lang::C => todo!(),
        Lang::Cpp => todo!(),
        Lang::Python => todo!(),
    }
    let config_path = args.path.join("firefly.toml");
    std::fs::write(config_path, CONFIG).context("write config")?;
    Ok(())
}

fn parse_lang(lang: &str) -> Result<Lang> {
    let result = match lang.to_lowercase().as_str() {
        "c" => Lang::C,
        "go" | "golang" => Lang::Go,
        "rust" | "rs" => Lang::Rust,
        "zig" => Lang::Zig,
        "ts" | "typescript" => Lang::TS,
        "cpp" | "c++" => Lang::Cpp,
        "python" | "py" => Lang::Python,
        _ => bail!("unsupported language: {lang}"),
    };
    Ok(result)
}

fn new_rust(path: &Path) -> Result<()> {
    let mut c = Commander::default();
    c.run(&["cargo", "new", path_to_utf8(path)?])?;
    c.cd(path)?;
    c.run(&["cargo", "add", "firefly_rust"])?;
    Ok(())
}

#[derive(Default)]
struct Commander<'a> {
    root: Option<&'a Path>,
}

impl<'a> Commander<'a> {
    fn cd(&mut self, path: &'a Path) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir(path)?;
        }
        self.root = Some(path);
        Ok(())
    }

    fn run(&self, a: &[&str]) -> Result<()> {
        let bin = a[0];
        let mut cmd = Command::new(bin);
        let mut cmd = &mut cmd;
        cmd = cmd.args(&a[1..]);
        if let Some(path) = self.root {
            cmd = cmd.current_dir(path);
        }
        let output = cmd.output().context(format!("run {bin}"))?;
        check_output(&output)?;
        Ok(())
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

/// Convert a file system path to UTF-8 if possible.
fn path_to_utf8(path: &Path) -> anyhow::Result<&str> {
    match path.to_str() {
        Some(path) => Ok(path),
        None => bail!("project root path cannot be converted to UTF-8"),
    }
}
