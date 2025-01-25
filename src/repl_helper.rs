use crate::args::Commands;
use clap::Subcommand;
use crossterm::style::Stylize;
use rustyline::highlight::CmdKind;
use rustyline::hint::Hint;
use rustyline::Context;
use std::borrow::Cow;

/// Helper is a struct that provides autocomplete and syntax highlighting for rustyline.
pub struct Helper {
    hints: Vec<CommandHint>,
}

impl Helper {
    pub fn new() -> Self {
        let mut hints = Vec::new();
        let cmds = [
            // commands
            "build", "export", "import", "vfs", "cheat", "monitor", "key", "catalog",
            //
            // subcommands
            "new", "add", "pub", "priv", "rm", "list", "show",
            //
            // aliases
            "install", "generate", "remove", "app", "author", "ls",
        ];
        for cmd in cmds {
            let h = CommandHint(cmd.to_string());
            hints.push(h);
        }
        Self { hints }
    }
}

// These traits are required to be implemented for the type
// to be able to pass it into `Editor.set_helper`.
impl rustyline::validate::Validator for Helper {}
impl rustyline::Helper for Helper {}

// Provides a very basic syntax highlighting for all user input in the REPL.
impl rustyline::highlight::Highlighter for Helper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let args: Vec<_> = line.split_ascii_whitespace().collect();
        let Some(cmd) = args.first() else {
            return Cow::Borrowed(line);
        };
        if Commands::has_subcommand(cmd) {
            let colored = cmd.blue().to_string();
            let line = line.replacen(cmd, &colored, 1);
            return Cow::Owned(line);
        }
        Cow::Borrowed(line)
    }

    // We make this method to always return true,
    // so that syntax highlighting kicks in every time the user presses a button.
    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true
    }
}

// Implement a very basic autocomplete.
impl rustyline::completion::Completer for Helper {
    type Candidate = CommandHint;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let mut res: Vec<CommandHint> = Vec::new();
        // Autocomplete only if the cursor is at the very end of the input string.
        if line.is_empty() || pos < line.len() {
            return Ok((pos, res));
        }

        // Take the last word and try to find all known names starting with it.
        let (_, word) = line.rsplit_once(' ').unwrap_or(("", line));
        for hint in &self.hints {
            if hint.display().starts_with(word) {
                res.push(hint.suffix(word.len()));
            }
        }
        Ok((pos, res))
    }
}

// Everything below is pretty much copy-pasted from an example in the rustyline repo.
//
// https://github.com/kkawakam/rustyline/blob/master/examples/diy_hints.rs
impl rustyline::hint::Hinter for Helper {
    type Hint = CommandHint;
}

#[derive(Hash, Debug, PartialEq, Eq)]
pub struct CommandHint(String);

impl CommandHint {
    fn suffix(&self, strip_chars: usize) -> Self {
        Self(self.0[strip_chars..].to_string())
    }
}

impl Hint for CommandHint {
    fn display(&self) -> &str {
        &self.0
    }

    fn completion(&self) -> Option<&str> {
        Some(&self.0)
    }
}

impl rustyline::completion::Candidate for CommandHint {
    fn display(&self) -> &str {
        &self.0
    }

    fn replacement(&self) -> &str {
        &self.0
    }
}
