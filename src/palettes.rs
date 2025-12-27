use anyhow::{bail, Context, Result};
use image::Rgb;
use std::collections::HashMap;

pub type Color = Option<Rgb<u8>>;
pub type Palette = [Color; 16];
pub type Palettes = HashMap<String, Palette>;
type RawPalette = HashMap<String, u32>;

/// The default color palette (SWEETIE-16).
///
/// <https://lospec.com/palette-list/sweetie-16>
/// <https://github.com/nesbox/TIC-80/wiki/Palette>
pub static SWEETIE16: &Palette = &[
    Some(Rgb([0x1a, 0x1c, 0x2c])), // #1a1c2c: black
    Some(Rgb([0x5d, 0x27, 0x5d])), // #5d275d: purple
    Some(Rgb([0xb1, 0x3e, 0x53])), // #b13e53: red
    Some(Rgb([0xef, 0x7d, 0x57])), // #ef7d57: orange
    Some(Rgb([0xff, 0xcd, 0x75])), // #ffcd75: yellow
    Some(Rgb([0xa7, 0xf0, 0x70])), // #a7f070: light green
    Some(Rgb([0x38, 0xb7, 0x64])), // #38b764: green
    Some(Rgb([0x25, 0x71, 0x79])), // #257179: dark green
    Some(Rgb([0x29, 0x36, 0x6f])), // #29366f: dark blue
    Some(Rgb([0x3b, 0x5d, 0xc9])), // #3b5dc9: blue
    Some(Rgb([0x41, 0xa6, 0xf6])), // #41a6f6: light blue
    Some(Rgb([0x73, 0xef, 0xf7])), // #73eff7: cyan
    Some(Rgb([0xf4, 0xf4, 0xf4])), // #f4f4f4: white
    Some(Rgb([0x94, 0xb0, 0xc2])), // #94b0c2: light gray
    Some(Rgb([0x56, 0x6c, 0x86])), // #566c86: gray
    Some(Rgb([0x33, 0x3c, 0x57])), // #333c57: dark gray
];

/// The PICO-8 color palette.
///
/// <https://nerdyteachers.com/PICO-8/Guide/PALETTES>
static PICO8: &Palette = &[
    Some(Rgb([0x00, 0x00, 0x00])), // #000000: black
    Some(Rgb([0x1D, 0x2B, 0x53])), // #1D2B53: dark blue
    Some(Rgb([0x7E, 0x25, 0x53])), // #7E2553: dark purple
    Some(Rgb([0x00, 0x87, 0x51])), // #008751: dark green
    Some(Rgb([0xAB, 0x52, 0x36])), // #AB5236: brown
    Some(Rgb([0x5F, 0x57, 0x4F])), // #5F574F: dark gray
    Some(Rgb([0xC2, 0xC3, 0xC7])), // #C2C3C7: light gray
    Some(Rgb([0xFF, 0xF1, 0xE8])), // #FFF1E8: white
    Some(Rgb([0xFF, 0x00, 0x4D])), // #FF004D: red
    Some(Rgb([0xFF, 0xA3, 0x00])), // #FFA300: orange
    Some(Rgb([0xFF, 0xEC, 0x27])), // #FFEC27: yellow
    Some(Rgb([0x00, 0xE4, 0x36])), // #00E436: green
    Some(Rgb([0x29, 0xAD, 0xFF])), // #29ADFF: blue
    Some(Rgb([0x83, 0x76, 0x9C])), // #83769C: indigo
    Some(Rgb([0xFF, 0x77, 0xA8])), // #FF77A8: pink
    Some(Rgb([0xFF, 0xCC, 0xAA])), // #FFCCAA: peach
];

/// The Kirokaze Gameboy color palette.
///
/// <https://lospec.com/palette-list/kirokaze-gameboy>
static GAMEBOY: &Palette = &[
    Some(Rgb([0x33, 0x2c, 0x50])), // #332c50: purple
    Some(Rgb([0x46, 0x87, 0x8f])), // #46878f: blue
    Some(Rgb([0x94, 0xe3, 0x44])), // #94e344: green
    Some(Rgb([0xe2, 0xf3, 0xe4])), // #e2f3e4: white
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
];

pub fn parse_palettes(raws: Option<&HashMap<String, RawPalette>>) -> Result<Palettes> {
    let mut palettes = Palettes::new();
    let Some(raws) = raws else {
        return Ok(palettes);
    };
    for (name, raw) in raws {
        let palette = parse_palette(raw).context(format!("parse {name} palette"))?;
        palettes.insert(name.clone(), palette);
    }
    Ok(palettes)
}

fn parse_palette(raw: &RawPalette) -> Result<Palette> {
    let len = raw.len();
    if len > 16 {
        bail!("too many colors")
    }
    if len < 2 {
        bail!("too few colors")
    }
    if raw.get("0").is_some() {
        bail!("color IDs must start at 1");
    }
    let len = u16::try_from(len).unwrap();

    let mut palette: Palette = Palette::default();
    for id in 1u16..=len {
        let Some(raw_color) = raw.get(&id.to_string()) else {
            bail!("color IDs must be consecutive but ID {id} is missing");
        };
        let color = parse_color(*raw_color)?;
        let idx = usize::from(id - 1);
        palette[idx] = color;
    }
    Ok(palette)
}

#[expect(clippy::cast_possible_truncation)]
fn parse_color(raw: u32) -> Result<Color> {
    if raw > 0xff_ff_ff {
        bail!("the color is out of range")
    }
    let r = (raw >> 16) as u8;
    let g = (raw >> 8) as u8;
    let b = raw as u8;
    Ok(Some(Rgb([r, g, b])))
}

pub fn get_palette<'a>(name: Option<&str>, palettes: &'a Palettes) -> Result<&'a Palette> {
    let Some(name) = name else {
        return Ok(SWEETIE16);
    };
    let Some(palette) = palettes.get(name) else {
        return get_builtin_palette(name);
    };
    Ok(palette)
}

pub fn get_builtin_palette(name: &str) -> Result<&'static Palette> {
    let name = name.to_ascii_lowercase();
    let palette = match name.as_str() {
        "sweetie16" | "sweetie-16" | "tic80" | "tic-80" | "default" => SWEETIE16,
        "pico" | "pico8" | "pico-8" => PICO8,
        "gameboy" | "game-boy" | "gb" | "kirokaze" => GAMEBOY,
        _ => bail!("palette {name} not found"),
    };
    Ok(palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_palettes() {
        let mut p = RawPalette::new();
        p.insert("1".to_string(), 0x_ff_00_00);
        p.insert("2".to_string(), 0x_00_ff_00);
        p.insert("3".to_string(), 0x_00_00_ff);
        let mut ps = HashMap::new();
        ps.insert("rgb".to_string(), p);
        let res = parse_palettes(Some(&ps)).unwrap();
        assert_eq!(res.len(), 1);
        let exp: Palette = [
            Some(Rgb([0xff, 0x00, 0x00])),
            Some(Rgb([0x00, 0xff, 0x00])),
            Some(Rgb([0x00, 0x00, 0xff])),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ];
        assert_eq!(*res.get("rgb").unwrap(), exp);
    }

    #[test]
    fn test_get_palette() {
        let mut p = Palettes::new();
        p.insert("sup".to_string(), *SWEETIE16);
        assert_eq!(get_palette(None, &p).unwrap(), SWEETIE16);
        assert_eq!(get_palette(Some("sup"), &p).unwrap(), SWEETIE16);
        assert_eq!(get_palette(Some("sweetie16"), &p).unwrap(), SWEETIE16);
        assert!(get_palette(Some("foobar"), &p).is_err());
    }
}
