use crate::args::NewArgs;
use crate::config::Lang;
use crate::langs::{check_installed, check_output};
use anyhow::{bail, Context, Ok, Result};
use rust_embed::Embed;
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

/// Bootstrap a new project.
pub fn cmd_new(args: &NewArgs) -> Result<()> {
    if let Err(err) = firefly_types::validate_id(&args.name) {
        bail!("invalid project name: {err}");
    }
    let root = Path::new(&args.name);
    if root.exists() {
        bail!("the directory already exists");
    }
    let lang = parse_lang(&args.lang)?;
    match lang {
        Lang::Go => new_go(&args.name).context("new Go project")?,
        Lang::Rust => new_rust(&args.name).context("new Rust project")?,
        Lang::Zig => new_zig(&args.name).context("new Zig project")?,
        Lang::TS => todo!("TypeScript is not supported yet"),
        Lang::C => new_c(&args.name).context("new C project")?,
        Lang::Cpp => new_cpp(&args.name).context("new C++ project")?,
        Lang::Python => todo!("Python is not supported yet"),
    }
    write_config(&args.name)?;
    init_git(&args.name)?;
    println!("✅ project created");
    Ok(())
}

/// Create and dump firefly.toml config.
fn write_config(name: &str) -> Result<()> {
    use std::fmt::Write;

    let root = Path::new(name);
    let config_path = root.join("firefly.toml");
    let username = get_username().unwrap_or_else(|| "joearms".to_string());

    let mut config = String::new();
    _ = writeln!(config, "author_id = \"{username}\"");
    _ = writeln!(config, "app_id = \"{name}\"");
    _ = writeln!(config, "author_name = \"{}\"", to_titlecase(&username));
    _ = writeln!(config, "app_name = \"{}\"", to_titlecase(name));

    std::fs::write(config_path, config).context("write config")?;
    Ok(())
}

/// Initialize git repository for the project.
fn init_git(name: &str) -> Result<()> {
    let root = Path::new(name);
    if root.join(".git").exists() {
        return Ok(());
    }
    let mut c = Commander::default();
    c.cd(name)?;
    c.run(&["git", "init", "-b", "main"])?;
    Ok(())
}

/// Convert `--lang` CLI flag into [`Lang`].
fn parse_lang(lang: &str) -> Result<Lang> {
    let result = match lang.to_lowercase().as_str() {
        "c" => Lang::C,
        "go" | "golang" => Lang::Go,
        "rust" | "rs" => Lang::Rust,
        "zig" => Lang::Zig,
        "ts" | "typescript" | "js" | "javascript" => Lang::TS,
        "cpp" | "c++" => Lang::Cpp,
        "python" | "py" => Lang::Python,
        _ => bail!("unsupported language: {lang}"),
    };
    Ok(result)
}

/// Create a new Rust project.
fn new_rust(name: &str) -> Result<()> {
    check_installed("Rust", "cargo", "version")?;
    let mut c = Commander::default();
    c.run(&["cargo", "new", name])?;
    c.cd(name)?;
    c.run(&["cargo", "add", "firefly_rust"])?;
    c.copy_asset(&["src", "main.rs"], "main.rs")?;

    {
        let path = Path::new(name).join("Cargo.toml");
        let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
        let asset = Assets::get("cargo.toml").unwrap();
        file.write_all(&asset.data)
            .context("append to Cargo.toml")?;
    }
    Ok(())
}

/// Create a new Zig project.
fn new_zig(name: &str) -> Result<()> {
    let mut c = Commander::default();
    c.cd(name)?;
    c.copy_asset(&["build.zig"], "build.zig")?;
    c.copy_asset(&["build.zig.zon"], "build.zig.zon")?;
    c.copy_asset(&["src", "main.zig"], "main.zig")?;
    Ok(())
}

/// Create a new Go project.
fn new_go(name: &str) -> Result<()> {
    check_installed("Go", "tinygo", "version")?;
    let mut c = Commander::default();
    c.cd(name)?;
    c.run(&["go", "mod", "init", name])?;
    c.run(&["go", "get", "github.com/firefly-zero/firefly-go"])?;
    c.copy_asset(&["main.go"], "main.go")?;
    c.run(&["go", "mod", "tidy"])?;
    Ok(())
}

/// Create a new C project.
fn new_c(name: &str) -> Result<()> {
    new_c_or_cpp(name, "main.c")
}

/// Create a new C++ project.
fn new_cpp(name: &str) -> Result<()> {
    new_c_or_cpp(name, "main.cpp")
}

fn new_c_or_cpp(name: &str, main: &str) -> Result<()> {
    const BASE_URL: &str = "https://github.com/firefly-zero/firefly-c/raw/refs/heads/main/src/";
    let mut c = Commander::default();
    c.cd(name)?;
    for fname in ["firefly.c", "firefly.h", "firefly_bindings.h"] {
        let url = &format!("{BASE_URL}{fname}");
        c.wget(&["vendor", "firefly", fname], url)?;
    }
    c.copy_asset(&[main], main)?;
    Ok(())
}

#[derive(Default)]
struct Commander<'a> {
    root: Option<&'a Path>,
}

impl<'a> Commander<'a> {
    fn cd(&mut self, name: &'a str) -> Result<()> {
        let path = Path::new(name);
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
        let mut reader = resp.into_body().into_reader();
        let mut full_path = self.root.unwrap().to_path_buf();
        for part in path {
            full_path = full_path.join(part);
        }
        let dir_path = full_path.parent().unwrap();
        std::fs::create_dir_all(dir_path).context("create dir")?;
        let mut writer = std::fs::File::create(full_path).context("create file")?;
        std::io::copy(&mut reader, &mut writer).context("save response")?;
        Ok(())
    }

    fn copy_asset(&self, path: &[&str], name: &str) -> Result<()> {
        let Some(src) = Assets::get(name) else {
            bail!("asset {name} not found")
        };
        let mut reader = &src.data[..];
        let mut full_path = self.root.unwrap().to_path_buf();
        for part in path {
            full_path = full_path.join(part);
        }
        let dir_path = full_path.parent().unwrap();
        std::fs::create_dir_all(dir_path).context("create dir")?;
        let mut writer = std::fs::File::create(full_path).context("create file")?;
        std::io::copy(&mut reader, &mut writer).context("save response")?;
        Ok(())
    }
}

/// Get username of the currently logged in user.
fn get_username() -> Option<String> {
    let username = std::env::var("USER").ok()?;
    if firefly_types::validate_id(&username).is_err() {
        return None;
    }
    Some(username)
}

/// Convert the given string to Title Case.
fn to_titlecase(s: &str) -> String {
    let mut result = String::new();
    let mut had_space = true;
    for char in s.chars() {
        if char == ' ' || char.is_ascii_punctuation() {
            result.push(' ');
            had_space = true;
            continue;
        }
        if had_space {
            result.push(char.to_ascii_uppercase());
            had_space = false;
            continue;
        }
        if char.is_ascii_uppercase() {
            result.push(' ');
        }
        result.push(char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_titlecase() {
        assert_eq!(to_titlecase("hello"), "Hello".to_string());
        assert_eq!(to_titlecase("Hello"), "Hello".to_string());
        assert_eq!(to_titlecase("hello-world"), "Hello World".to_string());
        assert_eq!(to_titlecase("hello world"), "Hello World".to_string());
        assert_eq!(to_titlecase("hello_world"), "Hello World".to_string());
        assert_eq!(to_titlecase("HelloWorld"), "Hello World".to_string());
        assert_eq!(to_titlecase("hello9"), "Hello9".to_string());
    }
}
