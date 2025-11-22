use crate::palettes::{Color, Palette};
use anyhow::{bail, Context, Result};
use image::{Pixel, Rgba, RgbaImage};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn convert_image(in_path: &Path, out_path: &Path, sys_pal: &Palette) -> Result<()> {
    let file = image::ImageReader::open(in_path).context("open image file")?;
    let img = file.decode().context("decode image")?;
    let img = img.to_rgba8();
    if img.width() % 8 != 0 {
        bail!("image width must be divisible by 8");
    }
    let mut img_pal = make_palette(&img, sys_pal).context("detect colors used in the image")?;
    let mut out = File::create(out_path).context("create output path")?;
    write_u8(&mut out, 0x21)?;
    let colors = img_pal.len();
    if colors <= 2 {
        extend_palette(&mut img_pal, sys_pal, 2);
        write_image::<1, 8>(out, &img, &img_pal, sys_pal).context("write 1BPP image")
    } else if colors <= 4 {
        extend_palette(&mut img_pal, sys_pal, 4);
        write_image::<2, 4>(out, &img, &img_pal, sys_pal).context("write 1BPP image")
    } else if colors <= 16 {
        extend_palette(&mut img_pal, sys_pal, 16);
        write_image::<4, 2>(out, &img, &img_pal, sys_pal).context("write 1BPP image")
    } else {
        let has_transparency = img_pal.iter().any(Option::is_none);
        if has_transparency && colors == 17 {
            bail!("cannot use all 16 colors with transparency, remove one color");
        }
        bail!("the image has too many colors");
    }
}

fn write_image<const BPP: u8, const PPB: usize>(
    mut out: File,
    img: &RgbaImage,
    img_pal: &[Color],
    sys_pal: &Palette,
) -> Result<()> {
    write_u8(&mut out, BPP)?; // BPP
    let Ok(width) = u16::try_from(img.width()) else {
        bail!("the image is too big")
    };
    write_u16(&mut out, width)?; // image width
    let transparent = pick_transparent(img_pal, sys_pal)?;
    write_u8(&mut out, transparent)?; // transparent color

    // palette swaps
    let mut byte = 0;
    debug_assert!(img_pal.len() == 2 || img_pal.len() == 4 || img_pal.len() == 16);
    for (i, color) in img_pal.iter().enumerate() {
        let index = match color {
            Some(color) => find_color(sys_pal, Some(*color)),
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
        let raw_color = find_color(img_pal, color);
        byte = (byte << BPP) | raw_color;
        if (i + 1) % PPB == 0 {
            write_u8(&mut out, byte)?;
        }
    }
    Ok(())
}

/// Detect all colors used in the image
fn make_palette(img: &RgbaImage, sys_pal: &Palette) -> Result<Vec<Color>> {
    let mut palette = Vec::new();
    for (x, y, pixel) in img.enumerate_pixels() {
        let color = convert_color(*pixel);
        if !palette.contains(&color) {
            if color.is_some() && !sys_pal.contains(&color) {
                bail!(
                    "found a color not present in the color palette: {} (at x={x}, y={y})",
                    format_color(color),
                );
            }
            palette.push(color);
        }
    }
    palette.sort_by_key(|c| match c {
        Some(c) => find_color(sys_pal, Some(*c)),
        None => 20,
    });
    Ok(palette)
}

/// Add empty colors at the end of the palette to match the BPP size.
fn extend_palette(img_pal: &mut Vec<Color>, sys_pal: &Palette, size: usize) {
    let n = size - img_pal.len();
    for _ in 0..n {
        img_pal.push(sys_pal[0]);
    }
}

fn write_u8(f: &mut File, v: u8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_u16(f: &mut File, v: u16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

/// Find the index of the given color in the given palette.
fn find_color(palette: &[Color], c: Color) -> u8 {
    for (color, i) in palette.iter().zip(0u8..) {
        if *color == c {
            return i;
        }
    }
    panic!("color not in the palette")
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
fn pick_transparent(img_pal: &[Color], sys_pal: &Palette) -> Result<u8> {
    if img_pal.iter().all(Option::is_some) {
        // no transparency needed
        return Ok(17);
    }
    for (color, i) in sys_pal.iter().zip(0u8..) {
        if !img_pal.contains(color) {
            return Ok(i);
        }
    }
    if img_pal.len() > 16 {
        bail!("the image cannot contain more than 16 colors")
    }
    if img_pal.len() == 16 {
        bail!("an image cannot contain all 16 colors and transparency")
    }
    bail!("image contains colors not from the palette")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::SWEETIE16;
    use image::Rgb;

    #[test]
    fn test_format_color() {
        assert_eq!(format_color(None), "ALPHA");
        assert_eq!(format_color(Some(Rgb([0x89, 0xab, 0xcd]))), "#89ABCD");
    }

    #[test]
    fn test_pick_transparent() {
        let pal = SWEETIE16;
        let c0 = pal[0];
        let c1 = pal[1];
        let c2 = pal[2];
        let c3 = pal[3];
        assert_eq!(pick_transparent(&[c0, c1], pal).unwrap(), 17);
        assert_eq!(pick_transparent(&[c0, c1, None], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c0, None, c1], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c1, c0, None], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c0, c1, c2, c3, None], pal).unwrap(), 4);
    }
}
