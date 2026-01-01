use crate::args::BoardsArgs;
use crate::file_names::BOARDS;
use anyhow::{Context, Result, bail};
use crossterm::style::Stylize;
use firefly_types::Encode;
use std::io::Read;
use std::path::Path;

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
        let scores = merge_scores(&friends, scores);
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
        let res = stream.read_exact(&mut buf[..1]);
        if res.is_err() {
            break;
        }
        let size = usize::from(buf[0]);
        if size > 16 {
            bail!("friend name is too long: {size} > 16");
        }
        stream.read_exact(&mut buf[1..=size]).context("read name")?;
        let name = &buf[1..=size];
        let name = std::str::from_utf8(name).context("decode name")?;
        friends.push(name.to_owned());
    }
    Ok(friends)
}

#[derive(PartialEq, Debug)]
struct Score {
    name: String,
    value: i16,
}

fn merge_scores(friends: &[String], scores: &firefly_types::BoardScores) -> Vec<Score> {
    let mut res = Vec::new();
    for score in scores.me.iter() {
        if *score == 0 {
            continue;
        }
        res.push(Score {
            name: "you".to_string(),
            value: *score,
        });
    }
    for score in scores.friends.iter() {
        if score.score == 0 {
            continue;
        }
        let name = match friends.get(usize::from(score.index)) {
            Some(name) => name.to_owned(),
            None => format!("friend #{}", score.index),
        };
        res.push(Score {
            name,
            value: score.score,
        });
    }
    res.sort_by_key(|s| -s.value);
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
    use crate::test_helpers::make_tmp_vfs;

    #[test]
    fn test_load_friends() {
        let vfs = make_tmp_vfs();
        let path = vfs.join("sys").join("friends");
        let contents: &[u8] = &[4, b'g', b'r', b'a', b'm', 1, b'!', 3, b'L', b'O', b'L'];
        std::fs::create_dir_all(vfs.join("sys")).unwrap();
        std::fs::write(path, contents).unwrap();
        let friends = load_friends(&vfs).unwrap();
        assert_eq!(friends[..], ["gram", "!", "LOL"]);
    }

    fn new_score(n: &'static str, v: i16) -> Score {
        Score {
            name: n.to_string(),
            value: v,
        }
    }

    #[test]
    fn test_merge_scores() {
        use firefly_types::*;
        let friends = ["alex".to_string(), "gram".to_string()];
        let scores = BoardScores {
            me: Box::new([40, 30, 20, 10, 9, 7, 0, 0]),
            friends: Box::new([
                FriendScore {
                    index: 0,
                    score: 44,
                },
                FriendScore {
                    index: 6,
                    score: 42,
                },
                FriendScore {
                    index: 1,
                    score: 37,
                },
                FriendScore {
                    index: 1,
                    score: 10,
                },
                FriendScore { index: 2, score: 8 },
                FriendScore { index: 0, score: 2 },
                FriendScore { index: 0, score: 0 },
                FriendScore { index: 0, score: 0 },
            ]),
        };
        let res = merge_scores(&friends[..], &scores);
        let exp = vec![
            new_score("alex", 44),
            new_score("friend #6", 42),
            new_score("you", 40),
            new_score("gram", 37),
            new_score("you", 30),
            new_score("you", 20),
            new_score("you", 10),
            new_score("gram", 10),
            new_score("you", 9),
            new_score("friend #2", 8),
            new_score("you", 7),
            new_score("alex", 2),
        ];
        assert_eq!(res.len(), exp.len());
        for (r, e) in res.iter().zip(&exp) {
            assert_eq!(r, e);
        }
        assert_eq!(res, exp);
    }

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
