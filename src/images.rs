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
    let img_pal = make_palette(&img, sys_pal).context("detect colors used in the image")?;
    let out = File::create(out_path).context("create output path")?;

    let n_colors = img_pal.len();
    if n_colors > 16 {
        let has_transparency = img_pal.iter().any(Option::is_none);
        if has_transparency && n_colors == 17 {
            bail!("cannot use all 16 colors with transparency, remove one color");
        }
        bail!("the image has too many colors");
    }

    let transp = pick_transparent(&img_pal, sys_pal)?;
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

/// Detect all colors used in the image.
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
        Some(c) => find_color(sys_pal, *c),
        None => 20,
    });
    Ok(palette)
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
        if *color == Some(c) {
            return i;
        }
    }
    panic!("color not in the palette")
}

/// Make human-readable hex representation of the color code.
fn format_color(c: Color) -> String {
    match c {
        Some(c) => {
            let c = c.0;
            format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
        }
        None => "TRANSPARENT".to_string(),
    }
}

fn convert_color(c: Rgba<u8>) -> Color {
    let alpha = c.0[3];
    let is_transparent = alpha < 128;
    if is_transparent {
        return None;
    }
    Some(c.to_rgb())
}

/// Pick the color to be used to represent transparency
fn pick_transparent(img_pal: &[Color], sys_pal: &Palette) -> Result<u8> {
    if img_pal.iter().all(Option::is_some) {
        return Ok(0xff); // no transparency needed
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
        bail!("the image cannot contain all 16 colors and transparency")
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
        assert_eq!(pick_transparent(&[c0, c1], pal).unwrap(), 255);
        assert_eq!(pick_transparent(&[c0, c1, None], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c0, None, c1], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c1, c0, None], pal).unwrap(), 2);
        assert_eq!(pick_transparent(&[c0, c1, c2, c3, None], pal).unwrap(), 4);
    }
}
