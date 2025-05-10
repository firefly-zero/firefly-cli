use crate::args::BadgesArgs;
use crate::file_names::BADGES;
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use firefly_types::Encode;
use std::path::Path;

pub fn cmd_badges(vfs: &Path, args: &BadgesArgs) -> Result<()> {
    let Some((author_id, app_id)) = args.id.split_once('.') else {
        bail!("invalid app id: dot not found");
    };

    // read stats
    let stats_path = vfs.join("data").join(author_id).join(app_id).join("stats");
    let raw = std::fs::read(stats_path).context("read stats file")?;
    let stats = firefly_types::Stats::decode(&raw).context("decode stats")?;

    // read badges
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
    let mut badges: Vec<_> = badges.badges.iter().zip(1..).collect();
    badges.sort_by_key(|(badge, _id)| badge.position);

    display_badges(&badges, &stats, args)
}

fn display_badges(
    badges: &[(&firefly_types::Badge<'_>, usize)],
    stats: &firefly_types::Stats,
    args: &BadgesArgs,
) -> Result<()> {
    for (badge, id) in badges {
        let Some(progress) = stats.badges.get(id - 1) else {
            bail!("there are fewer badges in stats file than in the rom");
        };
        if progress.done < badge.hidden {
            if !args.hidden {
                continue;
            }
            print!("{}", "[hidden] ".grey());
        }
        println!("#{id} {} ({} XP)", badge.name.cyan(), badge.xp);
        println!("{}", badge.descr);
        let emoji = if progress.earned() {
            "✅"
        } else if progress.done == 0 {
            "🚫"
        } else {
            "⌛"
        };
        println!("{emoji} {}/{}", progress.done, progress.goal);
        println!();
    }
    Ok(())
}
