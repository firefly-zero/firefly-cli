use crate::args::NameSetArgs;
use crate::vfs::generate_valid_name;
use anyhow::{bail, Context, Result};
use firefly_types::Encode;
use std::fs;
use std::path::Path;

pub fn cmd_name_get(vfs: &Path) -> Result<()> {
    let name_path = vfs.join("sys").join("name");
    let name = fs::read_to_string(name_path)?;
    if let Err(err) = firefly_types::validate_id(&name) {
        println!("⚠️  the name is not valid: {err}");
    }
    println!("{name}");
    Ok(())
}

pub fn cmd_name_set(vfs: &Path, args: &NameSetArgs) -> Result<()> {
    let name_path = vfs.join("sys").join("name");
    let old_name = fs::read_to_string(&name_path)?;
    println!("old name: {old_name}");
    if let Err(err) = firefly_types::validate_id(&args.name) {
        bail!("validate new name: {err}");
    }
    write_name(vfs, &args.name)?;
    println!("new name: {}", &args.name);
    Ok(())
}

pub fn cmd_name_generate(vfs: &Path) -> Result<()> {
    let name = generate_valid_name();
    write_name(vfs, &name)?;
    println!("new name: {name}");
    Ok(())
}

fn write_name(vfs: &Path, name: &str) -> Result<()> {
    let name_path = vfs.join("sys").join("name");
    fs::write(name_path, name)?;
    let settings_path = vfs.join("sys").join("config");
    let raw = fs::read(&settings_path).context("read settings")?;
    let mut settings = firefly_types::Settings::decode(&raw[..]).context("parse settings")?;
    settings.name = name.to_string();
    let raw = settings.encode_vec().context("encode settings")?;
    fs::write(settings_path, raw).context("write settings file")?;
    Ok(())
}
