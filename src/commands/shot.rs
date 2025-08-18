use anyhow::{bail, Result};
use std::{io::Write, path::Path};

use crate::args::ShotArgs;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 180;
const SIZE: usize = 1 + 48 + (180 * 240 / 2);

/// Download screenshot.
pub fn cmd_shot(vfs: &Path, args: &ShotArgs) -> Result<()> {
    todo!()
}

/// Convert raw screenshot file into a PNG file.
fn to_png(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.len() != SIZE {
        bail!("invalid file size: got {}, expected {SIZE}", raw.len());
    }
    if raw[0] != 0x41 {
        bail!("invalid magic number");
    }
    let palette: [u8; 48] = raw[1..0x32].try_into().unwrap();
    let frame = &raw[0x32..];

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
