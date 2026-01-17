use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MsgKind {
    Info,
    Progress1,
    Progress2,
    Warning,
    Success,
    Plain,
}

impl MsgKind {
    pub const fn emoji(self) -> &'static str {
        match self {
            Self::Info => "ℹ️",
            Self::Progress1 => "⏳️",
            Self::Progress2 => "⌛",
            Self::Warning => "⚠️",
            Self::Success => "✅",
            Self::Plain => "",
        }
    }
}

pub trait Env {
    fn emit_msg(&mut self, kind: MsgKind, msg: &str);
    fn vfs_path(&mut self) -> PathBuf;
}

pub struct StdEnv {
    pub vfs: PathBuf,
}

impl StdEnv {
    pub const fn new(vfs: PathBuf) -> Self {
        Self { vfs }
    }
}

impl Env for StdEnv {
    fn emit_msg(&mut self, kind: MsgKind, msg: &str) {
        if kind == MsgKind::Plain {
            println!("{msg}");
        } else {
            println!("{}  {msg}", kind.emoji());
        }
    }

    fn vfs_path(&mut self) -> PathBuf {
        self.vfs.clone()
    }
}
