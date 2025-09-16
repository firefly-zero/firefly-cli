use crate::args::ShotsDownloadArgs;
use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;
const SIZE: usize = 1 + 48 + (160 * 240 / 2);

/// Download screenshot from VFS.
pub fn cmd_shots_download(vfs: &Path, args: &ShotsDownloadArgs) -> Result<()> {
    let dst_dir: PathBuf = match &args.output {
        Some(dst_dir) => dst_dir.clone(),
        None => std::env::current_dir().context("get current dir")?,
    };

    // Handle absolute path or path relative to the current dir.
    let src_path = PathBuf::from(&args.source);
    if src_path.is_file() {
        return download_file(&src_path, &dst_dir);
    }
    if src_path.is_dir() {
        println!("downloading a dir from {}", path_to_utf8(&src_path));
        return download_dir(&src_path, &dst_dir);
    }

    // Handle path relative to the vfs root.
    if args.source.starts_with("data") {
        let src_path = vfs.join(&args.source);
        if src_path.is_file() {
            return download_file(&src_path, &dst_dir);
        }
        if src_path.is_dir() {
            return download_dir(&src_path, &dst_dir);
        }
    }

    // Handle full app ID (`lux.snek`).
    if let Some((author, app)) = args.source.split_once('.') {
        let src_dir = vfs.join("data").join(author).join(app).join("shots");
        if !src_dir.exists() {
            bail!("the app not found")
        }
        return download_dir(&src_dir, &dst_dir);
    }

    // Handle author ID (`lux`).
    let author_dir = vfs.join("data").join(&args.source);
    if author_dir.exists() {
        let dir = author_dir.read_dir().context("read author's data dir")?;
        for entry in dir {
            let entry = entry?;
            let src_dir = entry.path().join("shots");
            if src_dir.exists() {
                download_dir(&src_dir, &dst_dir)?;
            }
        }
        return Ok(());
    }

    bail!("source path not found")
}

fn download_dir(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    if dst_dir.is_file() || has_ext(dst_dir, "png") {
        bail!("source path is a dir but the destination path is a file");
    }
    println!(
        "⏳️ downloading all files from from {}...",
        path_to_utf8(src_dir)
    );
    if !dst_dir.exists() {
        std::fs::create_dir_all(dst_dir).context("create output dir")?;
    }
    if !src_dir.exists() {
        bail!("the source dir doesn't exist")
    }
    let dir = src_dir.read_dir().context("read source dir")?;
    for entry in dir {
        let entry = entry?;
        let src_path = entry.path();
        if !src_path.is_file() {
            continue;
        }
        let dst_file_name = get_output_file_name(&src_path)?;
        let dst_path = dst_dir.join(dst_file_name);
        copy_file(&src_path, &dst_path).with_context(|| {
            format!(
                "copy screenshot from {} into {}",
                path_to_utf8(&src_path),
                path_to_utf8(&dst_path),
            )
        })?;
    }
    Ok(())
}

/// Handle the command being invoked with a single file as input.
fn download_file(src_path: &Path, dst_path: &Path) -> Result<()> {
    println!(
        "⏳️ downloading a single file from {}...",
        path_to_utf8(src_path)
    );
    let is_file = has_ext(dst_path, "png");
    if dst_path.is_file() || is_file {
        // The output path is a file.
        copy_file(src_path, dst_path)
    } else {
        // The output path is a dir.
        if !dst_path.exists() {
            std::fs::create_dir_all(dst_path).context("create output dir")?;
        }
        let dst_file_name = get_output_file_name(src_path)?;
        let dst_path = dst_path.join(dst_file_name);
        copy_file(src_path, &dst_path)
    }
}

fn has_ext(path: &Path, ext: &str) -> bool {
    let Some(act) = path.extension() else {
        return false;
    };
    let Some(act) = act.to_str() else {
        return false;
    };
    act == ext
}

/// Given the path to the input screenshot file, generate output PNG file name.
fn get_output_file_name(src_path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for raw_part in src_path.components() {
        let raw_part = raw_part.as_os_str();
        if let Some(part) = raw_part.to_str() {
            parts.push(part);
        }
    }
    let Some((data_idx, _)) = parts.iter().enumerate().find(|(_, part)| **part == "data") else {
        bail!("cannot find data dir")
    };
    let author_id = parts[data_idx + 1];
    let app_id = parts[data_idx + 2];
    let file_name = parts[data_idx + 4];
    let Some((file_id, ext)) = file_name.split_once('.') else {
        bail!("file has no extension");
    };
    if ext != "ffs" && ext != "png" {
        bail!("invalid file extension *.{ext}, expected *.ffs");
    }
    Ok(format!("{author_id}.{app_id}.{file_id:0>3}.png"))
}

/// Read the input screenshot file and save it in output file as PNG.
fn copy_file(src_path: &Path, dst_path: &Path) -> Result<()> {
    // The old Firefly runtime versions used to save files as PNG.
    if has_ext(src_path, "png") {
        std::fs::copy(src_path, dst_path)?;
        return Ok(());
    }
    let src_raw = std::fs::read(src_path)?;
    let png_raw = to_png(&src_raw)?;
    std::fs::write(dst_path, png_raw)?;
    Ok(())
}

/// Convert raw screenshot file into a PNG file.
fn to_png(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.len() != SIZE {
        bail!("invalid file size: got {}, expected {SIZE}", raw.len());
    }
    if raw[0] != 0x41 {
        bail!("invalid magic number");
    }
    let palette: [u8; 48] = raw[1..0x31].try_into().unwrap();
    let frame = &raw[0x31..];

    let mut w = Vec::new();
    w.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;
    let mut ihdr: [u8; 13] = [0; 13];
    ihdr[..4].copy_from_slice(&WIDTH.to_be_bytes());
    ihdr[4..8].copy_from_slice(&HEIGHT.to_be_bytes());
    ihdr[8] = 4; // bit depth: 4 BPP
    ihdr[9] = 3; // color type: indexed (uses palette)
    write_chunk(&mut w, b"IHDR", &ihdr)?;
    write_chunk(&mut w, b"PLTE", &palette)?;
    write_frame(&mut w, frame)?;
    write_chunk(&mut w, b"IEND", &[])?;
    Ok(w)
}

/// Write the compressed PNG image data.
fn write_frame<W: Write>(mut w: W, data: &[u8]) -> Result<()> {
    let inner = Vec::new();
    let mut compressor = libflate::zlib::Encoder::new(inner).unwrap();
    for line in data.chunks(WIDTH as usize / 2) {
        compressor.write_all(&[0]).unwrap(); // filter type: no filter
        compressor.write_all(&swap_pairs(line)).unwrap();
    }
    let compressed = compressor.finish().into_result().unwrap();
    write_chunk(&mut w, b"IDAT", &compressed)?;
    Ok(())
}

/// Each byte in the frame buffer contains 2 pixels. Swap these 2 pixels.
///
/// We need to do it because firefly uses little-endian for everything
/// but PNG is big-endian.
fn swap_pairs(frame: &[u8]) -> Vec<u8> {
    frame.iter().map(|byte| byte.rotate_left(4)).collect()
}

/// Write a PNG chunk.
#[expect(clippy::trivially_copy_pass_by_ref)]
fn write_chunk<W: Write>(mut w: W, name: &[u8; 4], data: &[u8]) -> Result<()> {
    #[expect(clippy::cast_possible_truncation)]
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(name)?;
    w.write_all(data)?;
    let mut crc = crc32fast::Hasher::new();
    crc.update(name);
    crc.update(data);
    w.write_all(&crc.finalize().to_be_bytes())?;
    Ok(())
}

/// Convert a file system path to UTF-8 if possible.
pub fn path_to_utf8(path: &Path) -> &str {
    path.to_str().unwrap_or("???")
}
