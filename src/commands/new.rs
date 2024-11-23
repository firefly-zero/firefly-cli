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
        Lang::C => new_c(&args.path).context("new C project")?,
        Lang::Cpp => new_c(&args.path).context("new C++ project")?,
        Lang::Python => todo!(),
    }
    let config_path = args.path.join("firefly.toml");
    std::fs::write(config_path, CONFIG).context("write config")?;
    println!("✅ project created");
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

fn new_c(path: &Path) -> Result<()> {
    const BASE_URL: &str = "https://github.com/firefly-zero/firefly-c/raw/refs/heads/main/src/";
    let mut c = Commander::default();
    c.cd(path)?;
    for fname in ["firefly.c", "firefly.h", "firefly_bindings.h"] {
        let url = &format!("{BASE_URL}{fname}");
        c.wget(&["vendor", "firefly", fname], url)?;
    }
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

    /// Run a command.
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

    /// Download a file from the give URL and save it into the given path.
    fn wget(&self, path: &[&str], url: &str) -> Result<()> {
        let resp = ureq::get(url).call().context("send request")?;
        let mut reader = resp.into_reader();
        let mut full_path = self.root.unwrap().to_path_buf();
        for part in path {
            full_path = full_path.join(part);
        }
        let dir_path = full_path.parent().unwrap();
        std::fs::create_dir_all(dir_path).context("create dir")?;
        let mut writer = std::fs::File::create(full_path).context("create file")?;
        std::io::copy(&mut reader, &mut writer).context("save response")?;
        todo!()
    }
}

fn get_dir_name(path: &Path) -> anyhow::Result<&str> {
    let name = path.file_name().context("get directory name")?;
    let Some(name) = name.to_str() else {
        bail!("project name cannot be converted to UTF-8");
    };
    Ok(name)
}
