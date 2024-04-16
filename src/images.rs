use anyhow::{bail, Context};
use image::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) fn convert_image(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let file = image::io::Reader::open(input_path).context("open image file")?;
    let img = file.decode().context("decode image")?;
    let img = img.to_luma8();
    if img.width() % 8 != 0 {
        bail!("image width must be divisible by 8");
    }
    let palette = make_palette(&img).context("detect colors used in the image")?;
    let mut out = File::create(output_path).context("create output path")?;
    write_u8(&mut out, 0x21)?;
    if palette.len() <= 2 {
        write_image::<1, 8>(out, img, palette).context("write 1BPP image")
    } else {
        write_image::<2, 4>(out, img, palette).context("write 2BPP image")
    }
}

fn write_image<const BPP: usize, const PPB: usize>(
    mut out: File,
    img: ImageBuffer<Luma<u8>, Vec<u8>>,
    palette: heapless::Vec<u8, 4>,
) -> anyhow::Result<()> {
    write_u8(&mut out, BPP as u8)?;
    write_u16(&mut out, img.width() as u16)?;
    let mut byte: u8 = 0;
    for (i, pixel) in img.pixels().enumerate() {
        let luma = pixel.0[0];
        let raw_color = find_in_palette(&palette, luma);
        byte = (byte << BPP) | raw_color;
        if (i + 1) % PPB == 0 {
            write_u8(&mut out, byte)?;
            byte = 0;
        }
    }
    Ok(())
}

fn find_in_palette(palette: &heapless::Vec<u8, 4>, luma: u8) -> u8 {
    for (i, color) in palette.into_iter().enumerate() {
        if color == &luma {
            return i as u8;
        }
    }
    unreachable!("color is not in palette, palette is incomplete")
}

fn make_palette(img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> anyhow::Result<heapless::Vec<u8, 4>> {
    let mut palette = heapless::Vec::<u8, 4>::new();
    for pixel in img.pixels() {
        let raw = pixel.0[0];
        if !palette.contains(&raw) {
            let pushed = palette.push(raw);
            if pushed.is_err() {
                bail!("the image has more than 4 colors");
            };
        }
    }
    // darker colors usually go earlier in the palette
    palette.sort();
    Ok(palette)
}

fn write_u8(f: &mut File, v: u8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_u16(f: &mut File, v: u16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}
