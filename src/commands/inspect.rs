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
    let img = fs::read(path).ok()?;
    if img.len() < 8 {
        return None;
    }
    if img[0] != 0x21 {
        return None;
    }
    let fname = path.file_name()?;
    let fname: String = fname.to_str()?.to_string();
    Some(ImageStats {
        name: fname,
        bpp: img[1],
    })
}

fn print_meta(meta: &Meta) {
    println!("{}", "metadata".blue());
    println!("  {} {}", "author ID:   ".cyan(), meta.author_id);
    println!("  {} {}", "app ID:      ".cyan(), meta.app_id);
    println!("  {} {}", "author name: ".cyan(), meta.author_name);
    println!("  {} {}", "app name:    ".cyan(), meta.app_name);
    println!("  {} {}", "launcher:    ".cyan(), meta.launcher);
    println!("  {} {}", "sudo:        ".cyan(), meta.sudo);
    println!("  {} {}", "version:     ".cyan(), meta.version);
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
    println!("  {}", stats.name.clone().green());
    println!("    {}: {}", "bpp".cyan(), stats.bpp);
}
