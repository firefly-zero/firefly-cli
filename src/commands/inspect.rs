use crate::args::InspectArgs;
use crate::file_names::BIN;
use anyhow::Result;
use std::path::Path;

pub fn cmd_inspect(vfs: &Path, args: &InspectArgs) -> Result<()> {
    let id = match &args.id {
        Some(id) => id.to_owned(),
        None => detect_id()?,
    };
    let rom_path = vfs.join("roms").join(id);
    let bin_path = rom_path.join(BIN);
    println!("{bin_path:?}");
    Ok(())
}

fn detect_id() -> Result<String> {
    todo!()
}
