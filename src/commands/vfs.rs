use crate::vfs::get_vfs_path;

#[expect(clippy::unnecessary_wraps)]
pub fn cmd_vfs() -> anyhow::Result<()> {
    let path = get_vfs_path();
    let path = path.to_str().unwrap();
    println!("{path}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_cmd_vfs() {
        cmd_vfs().unwrap();
    }
}
