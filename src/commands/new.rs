use crate::args::NewArgs;
use crate::config::Lang;
use crate::langs::{check_output, path_to_utf8};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

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
        Lang::Go => new_go(&args.path).context("new Go project")?,
        Lang::Rust => new_rust(&args.path).context("new Rust project")?,
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

fn new_go(path: &Path) -> Result<()> {
    let mut c = Commander::default();
    c.cd(path)?;
    let name = get_dir_name(path)?;
    c.run(&["go", "mod", "init", name])?;
    c.run(&["go", "get", "github.com/firefly-zero/firefly-go"])?;
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
        check_output(&output).context(format!("run {bin}"))?;
        Ok(())
    }
}

fn get_dir_name(path: &Path) -> anyhow::Result<&str> {
    let name = path.file_name().context("get directory name")?;
    let Some(name) = name.to_str() else {
        bail!("project name cannot be converted to UTF-8");
    };
    Ok(name)
}
