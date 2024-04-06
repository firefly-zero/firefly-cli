use crate::error::CLIError;
use image::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) fn convert_image(input_path: &Path, output_path: &Path) -> Result<(), CLIError> {
    let img = image::io::Reader::open(input_path)?.decode()?;
    let img = img.to_luma8();
    let palette = make_palette(&img)?;
    let mut out = File::create(output_path)?;
    write_u8(&mut out, 0x21)?;
    if palette.len() <= 2 {
        dump_1bpp(out, img, palette)
    } else {
        dump_2bpp(out, img, palette)
    }
}

fn dump_1bpp(
    mut out: File,
    img: ImageBuffer<Luma<u8>, Vec<u8>>,
    palette: heapless::Vec<u8, 4>,
) -> Result<(), CLIError> {
    write_u8(&mut out, 0x00)?; // TODO: transparency and BPP
    write_u16(&mut out, img.width() as u16)?;
    todo!()
}

fn dump_2bpp(
    mut out: File,
    img: ImageBuffer<Luma<u8>, Vec<u8>>,
    palette: heapless::Vec<u8, 4>,
) -> Result<(), CLIError> {
    write_u8(&mut out, 0x00)?; // TODO: transparency and BPP
    write_u16(&mut out, img.width() as u16)?;
    for pixels in img.pixels().array_chunks::<4>() {
        todo!()
    }
    Ok(())
}

fn make_palette(img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Result<heapless::Vec<u8, 4>, CLIError> {
    let mut palette = heapless::Vec::<u8, 4>::new();
    for pixel in img.pixels() {
        let raw = pixel.0[0];
        if !palette.contains(&raw) {
            let pushed = palette.push(raw);
            if pushed.is_err() {
                return Err(CLIError::TooManyColors);
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
