use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MsgKind {
    Info,
    Progress1,
    Progress2,
    Warning,
    Success,
}

impl MsgKind {
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::Info => "ℹ️",
            Self::Progress1 => "⏳️",
            Self::Progress2 => "⌛",
            Self::Warning => "⚠️",
            Self::Success => "✅",
        }
    }
}

pub trait Env {
    fn emit_msg(&mut self, kind: MsgKind, msg: &str);
    fn vfs_path(&mut self) -> PathBuf;
}

pub struct StdEnv {
    pub vfs: PathBuf,
    had_msg: bool,
}

impl StdEnv {
    pub const fn new(vfs: PathBuf) -> Self {
        Self {
            vfs,
            had_msg: false,
        }
    }
}

impl Env for StdEnv {
    fn emit_msg(&mut self, kind: MsgKind, msg: &str) {
        if kind == MsgKind::Success && !self.had_msg {
            println!("{msg}");
            self.had_msg = true;
        }
        println!("{}  {msg}", kind.emoji());
        self.had_msg = true;
    }

    fn vfs_path(&mut self) -> PathBuf {
        self.vfs.clone()
    }
}
