use crate::{args::BoardsArgs, file_names::BOARDS};
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use firefly_types::Encode;
use std::{io::Read, path::Path};

pub fn cmd_boards(vfs: &Path, args: &BoardsArgs) -> Result<()> {
    let Some((author_id, app_id)) = args.id.split_once('.') else {
        bail!("invalid app id: dot not found");
    };

    // read stats
    let stats_path = vfs.join("data").join(author_id).join(app_id).join("stats");
    let raw = std::fs::read(stats_path).context("read stats file")?;
    let stats = firefly_types::Stats::decode(&raw).context("decode stats")?;

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
    let friends = load_friends(vfs).context("load list of friends")?;

    // display boards
    for (board, id) in boards {
        let Some(scores) = stats.scores.get(id - 1) else {
            bail!("there are fewer scores in stats file than boards in the rom");
        };
        println!("#{id} {}", board.name.cyan());
        let mut scores = merge_scores(&friends, scores);
        scores.sort_by_key(|s| s.value);
        for score in scores {
            if score.value > board.max {
                continue;
            }
            if score.value < board.min {
                continue;
            }
            let val = score.value.unsigned_abs();
            let val: String = if board.time {
                format_time(val)
            } else if board.decimals > 0 {
                format_decimal(val, board.decimals)
            } else {
                val.to_string()
            };
            let name: String = if &score.name == "me" {
                score.name.magenta().to_string()
            } else {
                score.name.clone()
            };
            println!("  {name:16} {val}");
        }
        println!();
    }
    Ok(())
}

fn load_friends(vfs: &Path) -> Result<Vec<String>> {
    let path = vfs.join("sys").join("friends");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut stream = std::fs::File::open(path).context("open sys/friends")?;
    let mut friends: Vec<String> = Vec::new();
    let mut buf = [0u8; 17];
    loop {
        let res = stream.read(&mut buf[..1]);
        if res.is_err() {
            break;
        }
        let size = usize::from(buf[0]);
        if size > 16 {
            bail!("friend name is too long: {size} > 16");
        }
        stream.read_exact(&mut buf[1..=size])?;
        let name = &buf[1..=size];
        let name = std::str::from_utf8(name)?;
        friends.push(name.to_owned());
    }
    Ok(friends)
}

struct Score {
    name: String,
    value: i16,
}

fn merge_scores(friends: &[String], scores: &firefly_types::BoardScores) -> Vec<Score> {
    let mut res = Vec::new();
    for score in scores.me.iter() {
        res.push(Score {
            name: "you".to_string(),
            value: *score,
        });
    }
    for score in scores.friends.iter() {
        let name = match friends.get(usize::from(score.index)) {
            Some(name) => name.to_owned(),
            None => format!("friend #{}", score.index),
        };
        res.push(Score {
            name,
            value: score.score,
        });
    }
    res
}

fn format_time(mut v: u16) -> String {
    let mut parts = Vec::new();
    while v > 0 {
        parts.push(format!("{:02}", v % 60));
        v /= 60;
    }
    parts.reverse();
    parts.join(":")
}

fn format_decimal(v: u16, prec: u8) -> String {
    let sep = (10u64).pow(prec.into());
    let right = u64::from(v) % sep;
    let left = u64::from(v) / sep;
    format!("{left}.{right:00$}", usize::from(prec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(1), "01".to_string());
        assert_eq!(format_time(13), "13".to_string());
        assert_eq!(format_time(60), "01:00".to_string());
        assert_eq!(format_time(62), "01:02".to_string());
        assert_eq!(format_time(143), "02:23".to_string());
        assert_eq!(format_time(13 * 3600 + 132), "13:02:12".to_string());
    }

    #[test]
    fn test_format_decimal() {
        assert_eq!(format_decimal(1341, 1), "134.1".to_string());
        assert_eq!(format_decimal(1341, 2), "13.41".to_string());
        assert_eq!(format_decimal(1341, 3), "1.341".to_string());
        assert_eq!(format_decimal(1341, 4), "0.1341".to_string());
        assert_eq!(format_decimal(1341, 5), "0.01341".to_string());
        assert_eq!(format_decimal(1341, 6), "0.001341".to_string());

        assert_eq!(format_decimal(13_001, 1), "1300.1".to_string());
        assert_eq!(format_decimal(13_001, 2), "130.01".to_string());
        assert_eq!(format_decimal(13_001, 3), "13.001".to_string());

        assert_eq!(format_decimal(13_010, 1), "1301.0".to_string());
        assert_eq!(format_decimal(13_010, 2), "130.10".to_string());
    }
}
