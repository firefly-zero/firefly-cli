use crate::palettes::{Color, Palette};
use anyhow::{Context, Result, bail};
use image::{Pixel, Rgb, Rgba, RgbaImage};
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
    let transp = find_unused_color(&img, sys_pal).context("detect colors used in the image")?;
    let out = File::create(out_path).context("create output path")?;
    write_image(out, &img, sys_pal, transp).context("write image")
}

fn write_image(mut out: File, img: &RgbaImage, sys_pal: &Palette, transp: u8) -> Result<()> {
    const BPP: u8 = 4;
    const PPB: usize = 2;

    let Ok(width) = u16::try_from(img.width()) else {
        bail!("the image is too big")
    };
    write_u8(&mut out, 0x22)?; // magic number
    write_u16(&mut out, width)?; // image width
    write_u8(&mut out, transp)?; // transparent color

    // Pixel values.
    let mut byte: u8 = 0;
    for (i, pixel) in img.pixels().enumerate() {
        let color = convert_color(*pixel);
        let raw_color = match color {
            Some(color) => find_color(sys_pal, color),
            None => transp,
        };
        byte = (byte << BPP) | raw_color;
        if (i + 1) % PPB == 0 {
            write_u8(&mut out, byte)?;
        }
    }
    Ok(())
}

/// Find color from the palette not used on the image.
///
/// Additionally ensures that the image uses the given color palette.
fn find_unused_color(img: &RgbaImage, sys_pal: &Palette) -> Result<u8> {
    let mut used_colors: Vec<Color> = Vec::new();
    let mut has_transp = false;
    for (x, y, pixel) in img.enumerate_pixels() {
        let Some(color) = convert_color(*pixel) else {
            has_transp = true;
            continue;
        };
        if !sys_pal.contains(&color) {
            bail!(
                "found a color not present in the color palette: {} (at x={x}, y={y})",
                format_color(color),
            );
        }
        if !used_colors.contains(&color) {
            used_colors.push(color);
        }
    }

    if has_transp {
        pick_transparent(&used_colors, sys_pal)
    } else {
        Ok(0xff)
    }
}

fn write_u8(f: &mut File, v: u8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_u16(f: &mut File, v: u16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

/// Find the index of the given color in the given palette.
fn find_color(palette: &Palette, c: Rgb<u8>) -> u8 {
    for (color, i) in palette.iter().zip(0u8..) {
        if *color == c {
            return i;
        }
    }
    panic!("color not in the palette")
}

/// Make human-readable hex representation of the color code.
fn format_color(c: Color) -> String {
    let c = c.0;
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn convert_color(c: Rgba<u8>) -> Option<Color> {
    let alpha = c.0[3];
    let is_transparent = alpha < 128;
    if is_transparent {
        return None;
    }
    Some(c.to_rgb())
}

/// Pick the color to be used to represent transparency
fn pick_transparent(img_pal: &[Color], sys_pal: &Palette) -> Result<u8> {
    assert!(img_pal.len() <= sys_pal.len());
    assert!(sys_pal.len() <= 16);
    for (color, i) in sys_pal.iter().zip(0u8..) {
        if !img_pal.contains(color) {
            return Ok(i);
        }
    }
    if sys_pal.len() == 16 {
        bail!("cannot use all 16 colors with transparency, remove one color");
    }
    // If the system palette has less than 16 colors,
    // any of the colors outside the palette
    // can be used for transparency. We use 15.
    Ok(0xf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::SWEETIE16;
    use image::Rgb;

    #[test]
    fn test_format_color() {
        assert_eq!(format_color(Rgb([0x89, 0xab, 0xcd])), "#89ABCD");
    }

    #[test]
    fn test_pick_transparent() {
        let pal = SWEETIE16;
        let c0 = pal[0];
        let c1 = pal[1];
        let c2 = pal[2];
        let c3 = pal[3];
        assert_eq!(pick_transparent(&[c0, c1], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c1, c0], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c0, c1, c2, c3], pal).unwrap(), 4);
    }
}
