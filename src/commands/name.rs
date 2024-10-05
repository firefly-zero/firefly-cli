use crate::{args::*, vfs::generate_valid_name};
use anyhow::{bail, Result};
use std::{fs, path::Path};

pub fn cmd_name_get(vfs: &Path, _args: &NameGetArgs) -> Result<()> {
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
    fs::write(name_path, &args.name)?;
    println!("new name: {}", &args.name);
    Ok(())
}

pub fn cmd_name_generate(vfs: &Path, _args: &NameGenerateArgs) -> Result<()> {
    let name_path = vfs.join("sys").join("name");
    let name = generate_valid_name();
    fs::write(name_path, &name)?;
    println!("new name: {name}");
    Ok(())
}
