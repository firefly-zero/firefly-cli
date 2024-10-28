use crate::{args::BoardsArgs, file_names::BOARDS};
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use firefly_types::Encode;
use std::path::Path;

pub fn cmd_boards(vfs: &Path, args: &BoardsArgs) -> Result<()> {
    let Some((author_id, app_id)) = args.id.split_once('.') else {
        bail!("invalid app id: dot not found");
    };

    // read boards
    let rom_path = vfs.join("roms").join(author_id).join(app_id);
    if !rom_path.exists() {
        bail!("app {author_id}.{app_id} is not installed");
    }
    let boards_path = rom_path.join(BOARDS);
    if !boards_path.exists() {
        bail!("the app does not have boards");
    }
    let raw = std::fs::read(boards_path).context("read boards file")?;
    let boards = firefly_types::Boards::decode(&raw).context("decode boards")?;
    let mut boards: Vec<_> = boards.boards.iter().zip(1..).collect();
    boards.sort_by_key(|(board, _id)| board.position);

    // display boards
    for (board, id) in &boards {
        println!("#{id} {}", board.name.cyan());
    }
    Ok(())
}
