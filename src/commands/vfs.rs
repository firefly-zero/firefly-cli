use crate::env::{Env, MsgKind};

#[expect(clippy::unnecessary_wraps)]
pub fn cmd_vfs<E: Env>(env: &mut E) -> anyhow::Result<()> {
    let path = env.vfs_path();
    let path = path.to_str().unwrap();
    env.emit_msg(MsgKind::Plain, path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::env::StdEnv;

    #[test]
    fn test_smoke_cmd_vfs() {
        let vfs = PathBuf::new().join("hello");
        let mut env = StdEnv::new(vfs);
        cmd_vfs(&mut env).unwrap();
    }
}
