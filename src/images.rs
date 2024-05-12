use anyhow::{bail, Context, Result};
use image::{Pixel, Rgb, Rgba, RgbaImage};
use std::fs::File;
use std::io::Write;
use std::path::Path;

type Color = Option<Rgb<u8>>;

static DEFAULT_PALETTE: &[Option<Rgb<u8>>] = &[
    // https://lospec.com/palette-list/sweetie-16
    // https://github.com/nesbox/TIC-80/wiki/Palette
    Some(Rgb([0x1a, 0x1c, 0x2c])), // black
    Some(Rgb([0x5d, 0x27, 0x5d])), // purple
    Some(Rgb([0xb1, 0x3e, 0x53])), // red
    Some(Rgb([0xef, 0x7d, 0x57])), // orange
    Some(Rgb([0xff, 0xcd, 0x75])), // yellow
    Some(Rgb([0xa7, 0xf0, 0x70])), // light green
    Some(Rgb([0x38, 0xb7, 0x64])), // green
    Some(Rgb([0x25, 0x71, 0x79])), // dark green
    Some(Rgb([0x29, 0x36, 0x6f])), // dark blue
    Some(Rgb([0x3b, 0x5d, 0xc9])), // blue
    Some(Rgb([0x41, 0xa6, 0xf6])), // light blue
    Some(Rgb([0x73, 0xef, 0xf7])), // cyan
    Some(Rgb([0xf4, 0xf4, 0xf4])), // white
    Some(Rgb([0x94, 0xb0, 0xc2])), // light gray
    Some(Rgb([0x56, 0x6c, 0x86])), // gray
    Some(Rgb([0x33, 0x3c, 0x57])), // dark gray
];

pub fn convert_image(input_path: &Path, output_path: &Path) -> Result<()> {
    let file = image::io::Reader::open(input_path).context("open image file")?;
    let img = file.decode().context("decode image")?;
    let img = img.to_rgba8();
    if img.width() % 8 != 0 {
        bail!("image width must be divisible by 8");
    }
    let palette = make_palette(&img).context("detect colors used in the image")?;
    let mut out = File::create(output_path).context("create output path")?;
    write_u8(&mut out, 0x21)?;
    let colors = palette.len();
    if colors <= 2 {
        let palette = extend_palette(palette, 2);
        write_image::<1, 8>(out, &img, &palette).context("write 1BPP image")
    } else if colors <= 4 {
        let palette = extend_palette(palette, 4);
        write_image::<2, 4>(out, &img, &palette).context("write 1BPP image")
    } else if colors <= 16 {
        let palette = extend_palette(palette, 16);
        write_image::<4, 2>(out, &img, &palette).context("write 1BPP image")
    } else {
        bail!("the image has too many colors")
    }
}

fn write_image<const BPP: u8, const PPB: usize>(
    mut out: File,
    img: &RgbaImage,
    palette: &[Color],
) -> Result<()> {
    write_u8(&mut out, BPP)?; // BPP
    let Ok(width) = u16::try_from(img.width()) else {
        bail!("the image is too big")
    };
    write_u16(&mut out, width)?; // image width
    let transparent = pick_transparent(palette)?;
    write_u8(&mut out, transparent)?; // transparent color

    // palette swaps
    let mut byte = 0;
    debug_assert!(palette.len() == 2 || palette.len() == 4 || palette.len() == 16);
    for (i, color) in palette.iter().enumerate() {
        let index = match color {
            Some(color) => find_color_default(*color),
            None => transparent,
        };
        byte = (byte << 4) | index;
        if i % 2 == 1 {
            write_u8(&mut out, byte)?;
        }
    }

    // image raw packed bytes
    let mut byte: u8 = 0;
    for (i, pixel) in img.pixels().enumerate() {
        let color = convert_color(*pixel);
        let raw_color = find_color(palette, color);
        byte = (byte << BPP) | raw_color;
        if (i + 1) % PPB == 0 {
            write_u8(&mut out, byte)?;
        }
    }
    Ok(())
}

/// Detect all colors used in the image
fn make_palette(img: &RgbaImage) -> Result<Vec<Color>> {
    let mut palette = Vec::new();
    for pixel in img.pixels() {
        let color = convert_color(*pixel);
        if !palette.contains(&color) {
            if color.is_some() && !DEFAULT_PALETTE.contains(&color) {
                bail!(
                    "found a color not present in the default color palette: {}",
                    format_color(color)
                );
            }
            palette.push(color);
        }
    }
    palette.sort_by_key(|c| match c {
        Some(c) => find_color_default(*c),
        None => 20,
    });
    Ok(palette)
}

/// Add empty colors at the end of the palette to match the BPP size.
fn extend_palette(mut palette: Vec<Color>, size: usize) -> Vec<Color> {
    let n = size - palette.len();
    for _ in 0..n {
        palette.push(DEFAULT_PALETTE[0]);
    }
    palette
}

fn write_u8(f: &mut File, v: u8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_u16(f: &mut File, v: u16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

/// Find the index of the given color in the default palette.
fn find_color_default(c: Rgb<u8>) -> u8 {
    find_color(DEFAULT_PALETTE, Some(c))
}

/// Find the index of the given color in the given palette.
fn find_color(palette: &[Color], c: Color) -> u8 {
    for (color, i) in palette.iter().zip(0u8..) {
        if *color == c {
            return i;
        }
    }
    panic!("color not in the default palette")
}

/// Make human-friendly hex representation of the color code.
fn format_color(c: Color) -> String {
    match c {
        Some(c) => {
            let c = c.0;
            format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
        }
        None => "ALPHA".to_string(),
    }
}

fn convert_color(c: Rgba<u8>) -> Color {
    if is_transparent(c) {
        return None;
    }
    Some(c.to_rgb())
}

const fn is_transparent(c: Rgba<u8>) -> bool {
    let alpha = c.0[3];
    alpha < 128
}

/// Pick the color to be used to represent transparency
fn pick_transparent(palette: &[Color]) -> Result<u8> {
    for (color, i) in DEFAULT_PALETTE.iter().zip(0u8..) {
        if !palette.contains(color) {
            return Ok(i);
        }
    }

    if palette.len() > 16 {
        bail!("the image cannot contain more than 16 colors")
    }
    if palette.len() == 16 {
        bail!("an image cannot contain all 16 colors and transparency")
    }
    bail!("image contains colors not from the default palette")
}
