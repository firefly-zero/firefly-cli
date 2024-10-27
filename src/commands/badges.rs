use crate::{args::BadgesArgs, file_names::BADGES};
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use firefly_types::Encode;
use std::path::Path;

pub fn cmd_badges(vfs: &Path, args: &BadgesArgs) -> Result<()> {
    let Some((author_id, app_id)) = args.id.split_once('.') else {
        bail!("invalid app id: dot not found");
    };
    let rom_path = vfs.join("roms").join(author_id).join(app_id);
    if !rom_path.exists() {
        bail!("app {author_id}.{app_id} is not installed");
    }
    let badges_path = rom_path.join(BADGES);
    if !badges_path.exists() {
        bail!("the app does not have badges");
    }
    let raw = std::fs::read(badges_path).context("read badges file")?;
    let badges = firefly_types::Badges::decode(&raw).context("decode badges")?;
    let mut badges = badges.badges.to_vec();
    badges.sort_by_key(|b| b.position);
    for badge in &badges {
        if badge.hidden && !args.hidden {
            continue;
        }
        if badge.hidden {
            print!("{}", "[hidden] ".grey());
        }
        println!("{} ({} XP)", badge.name.cyan(), badge.xp);
        println!("{}", badge.descr);
        println!("{}", badge.position);
        println!();
    }
    Ok(())
}
