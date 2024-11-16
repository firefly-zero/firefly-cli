use crossterm::style::Stylize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

/// Get size in bytes for every file in the ROM directory.
pub fn collect_sizes(root: &Path) -> HashMap<OsString, u64> {
    let mut sizes = HashMap::new();
    let Ok(entries) = fs::read_dir(root) else {
        return sizes;
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let Ok(meta) = entry.metadata() else { continue };
        sizes.insert(entry.file_name(), meta.len());
    }
    sizes
}

/// Convert big file size into Kb or Mb.
pub fn format_size(size: u64) -> String {
    if size > 1024 * 1024 {
        let new_size = size / 1024 / 1024;
        format!("{new_size:>5} {}", "Mb".magenta())
    } else if size > 1024 {
        let new_size = size / 1024;
        format!("{new_size:>5} {}", "Kb".blue())
    } else {
        format!("{size:>8}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "       0");
        assert_eq!(format_size(4), "       4");
        assert_eq!(format_size(1002), "    1002");
        assert_eq!(format_size(2000), "    1 \u{1b}[38;5;12mKb\u{1b}[39m");
        assert_eq!(format_size(2047), "    1 \u{1b}[38;5;12mKb\u{1b}[39m");
        assert_eq!(format_size(2048), "    2 \u{1b}[38;5;12mKb\u{1b}[39m");
        assert_eq!(format_size(2049), "    2 \u{1b}[38;5;12mKb\u{1b}[39m");
        assert_eq!(
            format_size(2 * 1024 * 1024),
            "    2 \u{1b}[38;5;13mMb\u{1b}[39m"
        );
    }
}
