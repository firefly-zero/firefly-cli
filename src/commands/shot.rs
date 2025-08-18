use anyhow::{bail, Context, Result};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

use crate::args::ShotArgs;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;
const SIZE: usize = 1 + 48 + (160 * 240 / 2);

/// Download screenshot.
pub fn cmd_shot(vfs: &Path, args: &ShotArgs) -> Result<()> {
    let dst_dir: PathBuf = match &args.output {
        Some(dst_dir) => dst_dir.clone(),
        None => std::env::current_dir().context("get current dir")?,
    };
    let sources = list_sources(vfs, &args.sources);
    for src_path in sources {
        let dst_file_name = get_output_file_name(&src_path)?;
        let dst_path = dst_dir.join(dst_file_name);
        copy_file(&src_path, &dst_path)?;
    }
    Ok(())
}

fn list_sources(vfs: &Path, sources: &[String]) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for src in sources {
        let path = vfs.join(src);
        if path.exists() {
            result.push(path);
        } else {
            let path = PathBuf::from(src);
            if path.exists() {
                result.push(path);
            } else {
                todo!();
            }
        }
    }
    result
}

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
    if ext != "ffs" {
        bail!("invalid file extension, expected .ffs");
    }
    Ok(format!("{author_id}.{app_id}.{file_id}.png"))
}

/// Read the input screenshot file and save it in output file as PNG.
fn copy_file(src_path: &Path, dst_path: &Path) -> Result<()> {
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
pub fn path_to_utf8(path: &Path) -> anyhow::Result<&str> {
    match path.to_str() {
        Some(path) => Ok(path),
        None => bail!("project root path cannot be converted to UTF-8"),
    }
}
