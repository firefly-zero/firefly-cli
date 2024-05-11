use anyhow::{bail, Context};
use image::{Rgb, RgbImage};
use std::fs::File;
use std::io::Write;
use std::path::Path;

static DEFAULT_PALETTE: &[Rgb<u8>] = &[
    // https://lospec.com/palette-list/sweetie-16
    // https://github.com/nesbox/TIC-80/wiki/Palette
    Rgb([0x1a, 0x1c, 0x2c]), // black
    Rgb([0x5d, 0x27, 0x5d]), // purple
    Rgb([0xb1, 0x3e, 0x53]), // red
    Rgb([0xef, 0x7d, 0x57]), // orange
    Rgb([0xff, 0xcd, 0x75]), // yellow
    Rgb([0xa7, 0xf0, 0x70]), // light green
    Rgb([0x38, 0xb7, 0x64]), // green
    Rgb([0x25, 0x71, 0x79]), // dark green
    Rgb([0x29, 0x36, 0x6f]), // dark blue
    Rgb([0x3b, 0x5d, 0xc9]), // blue
    Rgb([0x41, 0xa6, 0xf6]), // light blue
    Rgb([0x73, 0xef, 0xf7]), // cyan
    Rgb([0xf4, 0xf4, 0xf4]), // white
    Rgb([0x94, 0xb0, 0xc2]), // light gray
    Rgb([0x56, 0x6c, 0x86]), // gray
    Rgb([0x33, 0x3c, 0x57]), // dark gray
];

pub fn convert_image(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let file = image::io::Reader::open(input_path).context("open image file")?;
    let img = file.decode().context("decode image")?;
    let img = img.to_rgb8();
    if img.width() % 8 != 0 {
        bail!("image width must be divisible by 8");
    }
    let palette = make_palette(&img).context("detect colors used in the image")?;
    let mut out = File::create(output_path).context("create output path")?;
    write_u8(&mut out, 0x21)?;
    let colors = palette.len();
    if colors <= 2 {
        write_image::<1, 8>(out, &img, &palette).context("write 1BPP image")
    } else if colors <= 4 {
        write_image::<2, 4>(out, &img, &palette).context("write 1BPP image")
    } else if colors <= 16 {
        write_image::<4, 2>(out, &img, &palette).context("write 1BPP image")
    } else {
        bail!("the image has too many colors")
    }
}

fn write_image<const BPP: usize, const PPB: usize>(
    mut out: File,
    img: &RgbImage,
    palette: &[Rgb<u8>],
) -> anyhow::Result<()> {
    write_u8(&mut out, BPP as u8)?; // BPP
    write_u16(&mut out, img.width() as u16)?; // image width
    write_u8(&mut out, 0xff)?; // transparent color

    // palette swaps
    let mut byte = 0;
    for (i, color) in palette.iter().enumerate() {
        byte = (byte << 4) | find_color_default(color) as u8;
        if i % 2 == 1 {
            write_u8(&mut out, byte)?;
        }
    }

    // image raw packed bytes
    let mut byte: u8 = 0;
    for (i, pixel) in img.pixels().enumerate() {
        let raw_color = find_color(palette, pixel) as u8;
        byte = (byte << BPP) | raw_color;
        if (i + 1) % PPB == 0 {
            write_u8(&mut out, byte)?;
        }
    }
    Ok(())
}

/// Detect all colors used in the image
fn make_palette(img: &RgbImage) -> anyhow::Result<Vec<Rgb<u8>>> {
    let mut palette = Vec::new();
    for pixel in img.pixels() {
        if !palette.contains(pixel) {
            if !DEFAULT_PALETTE.contains(pixel) {
                bail!("found a color not present in the default color palette");
            }
            palette.push(*pixel);
        }
    }
    // darker colors usually go earlier in the palette
    palette.sort_by_key(|c| find_color(DEFAULT_PALETTE, c));
    Ok(palette)
}

fn write_u8(f: &mut File, v: u8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_u16(f: &mut File, v: u16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

/// Find the index of thfind_color_defaulte given color in the default palette.
fn find_color_default(c: &Rgb<u8>) -> usize {
    find_color(DEFAULT_PALETTE, c)
}

/// Find the index of the given color in the given palette.
fn find_color(palette: &[Rgb<u8>], c: &Rgb<u8>) -> usize {
    for (i, color) in palette.iter().enumerate() {
        if color == c {
            return i;
        }
    }
    panic!("color not in the default palette")
}
