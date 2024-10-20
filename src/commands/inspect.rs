use crate::args::InspectArgs;
use crate::config::Config;
use crate::file_names::{BIN, META};
use crate::fs::{collect_sizes, format_size};
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use firefly_types::Meta;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use wasmparser::Parser;
use wasmparser::Payload::*;

pub fn cmd_inspect(vfs: &Path, args: &InspectArgs) -> Result<()> {
    let (author_id, app_id) = get_id(vfs.to_path_buf(), args).context("get app ID")?;
    let rom_path = vfs.join("roms").join(&author_id).join(&app_id);
    if !rom_path.exists() {
        bail!("app {author_id}.{app_id} is not installed");
    }

    {
        let sizes = collect_sizes(&rom_path);
        print_sizes(&sizes);
    }
    {
        let meta_path = rom_path.join(META);
        let raw = fs::read(meta_path).context("read meta")?;
        let meta = Meta::decode(&raw).context("decode meta")?;
        print_meta(&meta);
    }
    {
        let bin_path = rom_path.join(BIN);
        let wasm_stats = inspect_wasm(&bin_path).context("inspect wasm binary")?;
        print_wasm_stats(&wasm_stats);
    }
    {
        let images_stats = inspect_images(&rom_path).context("inspect images")?;
        print_images_stats(&images_stats);
    }
    Ok(())
}

fn get_id(vfs: PathBuf, args: &InspectArgs) -> Result<(String, String)> {
    let res = if let Some(id) = &args.id {
        let Some((author_id, app_id)) = id.split_once('.') else {
            bail!("invalid app id: dot not found");
        };
        (author_id.to_string(), app_id.to_string())
    } else {
        let config = Config::load(vfs, &args.root).context("read project config")?;
        (config.author_id, config.app_id)
    };
    Ok(res)
}

#[derive(Default)]
struct WasmStats {
    imports: Vec<(String, String)>,
    exports: Vec<String>,
    memory: u64,
    globals: u32,
    functions: u32,
    code_size: u32,
    data_size: usize,
}

fn inspect_wasm(bin_path: &Path) -> anyhow::Result<WasmStats> {
    let parser = Parser::new(0);
    let mut stats = WasmStats::default();
    let input_bytes = std::fs::read(bin_path).context("read wasm binary")?;
    let input = parser.parse_all(&input_bytes);
    for payload in input {
        let payload = payload?;
        match payload {
            ImportSection(imports) => {
                for import in imports {
                    let import = import?;
                    let name = (import.module.to_owned(), import.name.to_owned());
                    stats.imports.push(name);
                }
            }
            GlobalSection(globals) => {
                stats.globals = globals.count();
            }
            MemorySection(memories) => {
                for memory in memories {
                    let memory = memory?;
                    stats.memory += memory.initial;
                }
            }
            ExportSection(exports) => {
                for export in exports {
                    let export = export?;
                    stats.exports.push(export.name.to_owned());
                }
            }
            DataSection(datas) => {
                for data in datas {
                    let data = data?;
                    stats.data_size += data.data.len();
                }
            }
            CodeSectionStart { count, size, .. } => {
                stats.code_size = size;
                stats.functions = count;
            }
            _ => {}
        }
    }
    stats.imports.sort();
    stats.exports.sort();
    Ok(stats)
}

struct ImageStats {
    name: String,
    bpp: u8,
    width: u16,
    height: u16,
    swaps: Vec<Option<u8>>,
    pixels: usize,
}

fn inspect_images(rom_path: &Path) -> anyhow::Result<Vec<ImageStats>> {
    let dir = fs::read_dir(rom_path)?;
    let mut stats = Vec::new();
    for entry in dir {
        let entry = entry?;
        if let Some(stat) = inspect_image(&entry.path()) {
            stats.push(stat);
        };
    }
    Ok(stats)
}

fn inspect_image(path: &Path) -> Option<ImageStats> {
    let image_bytes = fs::read(path).ok()?;
    if image_bytes.len() < 8 {
        return None;
    }
    if image_bytes[0] != 0x21 {
        return None;
    }
    let bpp = image_bytes[1];
    let width = u16::from(image_bytes[2]) | (u16::from(image_bytes[3]) << 8);
    let transp = image_bytes[4];
    let image_bytes = &image_bytes[5..];
    let swaps_len = match bpp {
        1 => 1,
        2 => 2,
        _ => 8,
    };
    let max_colors = match bpp {
        1 => 1,
        2 => 4,
        _ => 16,
    };
    let Some(swaps) = &image_bytes.get(..swaps_len) else {
        return None;
    };
    let image_bytes = &image_bytes[swaps_len..];
    let ppb = match bpp {
        1 => 8,
        2 => 4,
        _ => 2,
    };
    let pixels = image_bytes.len() * ppb;
    #[expect(clippy::cast_possible_truncation)]
    let height = pixels as u16 / width;
    let swaps = parse_swaps(transp, swaps);
    let swaps = swaps[..max_colors].to_vec();

    let name = path.file_name()?;
    let name: String = name.to_str()?.to_string();
    Some(ImageStats {
        name,
        bpp,
        width,
        height,
        swaps,
        pixels,
    })
}

fn print_meta(meta: &Meta) {
    println!("{}", "metadata".blue());
    println!("  {}:   {}", "author ID".cyan(), meta.author_id);
    println!("  {}:      {}", "app ID".cyan(), meta.app_id);
    println!("  {}: {}", "author name".cyan(), meta.author_name);
    println!("  {}:    {}", "app name".cyan(), meta.app_name);
    println!("  {}:    {}", "launcher".cyan(), meta.launcher);
    println!("  {}:        {}", "sudo".cyan(), meta.sudo);
    println!("  {}:     {}", "version".cyan(), meta.version);
    println!();
}

fn print_sizes(sizes: &HashMap<OsString, u64>) {
    println!("{}", "files".blue());
    let width = sizes.iter().map(|(n, _)| n.len()).max().unwrap_or_default();
    for (name, size) in sizes {
        let name = name.to_str().unwrap_or("???");
        let size = format_size(*size);
        println!("  {name:width$} {size}");
    }
    println!();
}

fn print_wasm_stats(stats: &WasmStats) {
    let code_size = format_size(stats.code_size.into());
    let code_size = code_size.trim_start();
    let data_size = format_size(stats.data_size as u64);
    let data_size = data_size.trim_start();

    println!("{}", "wasm binary".blue());
    println!("  {}: {}", "code size".cyan(), code_size);
    println!("  {}: {}", "data size".cyan(), data_size);
    println!("  {}: {}", "functions".cyan(), stats.functions);
    println!("  {}:   {}", "globals".cyan(), stats.globals);
    println!("  {}:    {} page(s)", "memory".cyan(), stats.memory);
    println!("  {}:   {}", "imports".cyan(), stats.imports.len());
    for (mod_name, func_name) in &stats.imports {
        let mod_name = mod_name.clone().magenta();
        println!("    {mod_name}.{func_name}");
    }
    println!("  {}:   {}", "exports".cyan(), stats.exports.len());
    for export in &stats.exports {
        // TODO: when we stabilize the list of callbacks, highlight unknown exports.
        println!("    {export}");
    }
}

fn print_images_stats(stats: &Vec<ImageStats>) {
    if stats.is_empty() {
        return;
    }
    println!();
    println!("{}", "images".blue());
    for stat in stats {
        print_image_stats(stat);
    }
}

fn print_image_stats(stats: &ImageStats) {
    println!("  {}", stats.name.clone().magenta());
    println!("    {}:    {}", "bpp".cyan(), stats.bpp);
    println!("    {}:  {}", "width".cyan(), stats.width);
    println!("    {}: {}", "height".cyan(), stats.height);
    println!("    {}: {}", "pixels".cyan(), stats.pixels);
    println!("    {}", "colors".cyan());
    for (i, swap) in stats.swaps.iter().enumerate() {
        if let Some(swap) = swap {
            let name = get_color_name(*swap);
            let swap = swap + 1;
            println!("      {i:>2} -> {swap:>2}  {name}");
        } else {
            println!("      {i:>2} ->  0  transparent");
        }
    }
}

fn parse_swaps(transp: u8, swaps: &[u8]) -> [Option<u8>; 16] {
    #[expect(clippy::get_first)]
    [
        // 0-4
        parse_color_l(transp, swaps.get(0)),
        parse_color_r(transp, swaps.get(0)),
        parse_color_l(transp, swaps.get(1)),
        parse_color_r(transp, swaps.get(1)),
        // 4-8
        parse_color_l(transp, swaps.get(2)),
        parse_color_r(transp, swaps.get(2)),
        parse_color_l(transp, swaps.get(3)),
        parse_color_r(transp, swaps.get(3)),
        // 8-12
        parse_color_l(transp, swaps.get(4)),
        parse_color_r(transp, swaps.get(4)),
        parse_color_l(transp, swaps.get(5)),
        parse_color_r(transp, swaps.get(5)),
        // 12-16
        parse_color_l(transp, swaps.get(6)),
        parse_color_r(transp, swaps.get(6)),
        parse_color_l(transp, swaps.get(7)),
        parse_color_r(transp, swaps.get(7)),
    ]
}

/// Parse the high bits of a byte as a color.
fn parse_color_r(transp: u8, c: Option<&u8>) -> Option<u8> {
    let c = c?;
    let c = c & 0b1111;
    if c == transp {
        return None;
    }
    Some(c)
}

/// Parse the low bits of a byte as a color.
fn parse_color_l(transp: u8, c: Option<&u8>) -> Option<u8> {
    let c = c?;
    let c = (c >> 4) & 0b1111;
    if c == transp {
        return None;
    }
    Some(c)
}

const fn get_color_name(swap: u8) -> &'static str {
    match swap {
        0 => "black        #1A1C2C",
        1 => "purple       #5D275D",
        2 => "red          #B13E53",
        3 => "orange       #EF7D57",
        4 => "yellow       #FFCD75",
        5 => "light green  #A7F070",
        6 => "green        #38B764",
        7 => "dark green   #257179",
        8 => "dark blue    #29366F",
        9 => "blue         #3B5DC9",
        10 => "light blue   #41A6F6",
        11 => "cyan         #73EFF7",
        12 => "white        #F4F4F4",
        13 => "light gray   #94B0C2",
        14 => "gray         #566C86",
        15 => "dark gray    #333C57",
        _ => "???",
    }
}
