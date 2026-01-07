use crate::args::InspectArgs;
use crate::config::Config;
use crate::file_names::{BIN, META};
use crate::fs::{collect_sizes, format_size};
use anyhow::{Context, Result, bail};
use crossterm::style::Stylize;
use firefly_types::{Encode, Meta};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use wasmparser::Payload::*;
use wasmparser::{Parser, Payload, Validator, WasmFeatures};

// https://github.com/wasmi-labs/wasmi/?tab=readme-ov-file#webassembly-features
const SUPPORTED_FEATURES: [WasmFeatures; 15] = [
    WasmFeatures::FLOATS,
    WasmFeatures::MUTABLE_GLOBAL,
    WasmFeatures::SATURATING_FLOAT_TO_INT,
    WasmFeatures::SIGN_EXTENSION,
    WasmFeatures::MULTI_VALUE,
    WasmFeatures::BULK_MEMORY,
    WasmFeatures::REFERENCE_TYPES,
    WasmFeatures::TAIL_CALL,
    WasmFeatures::EXTENDED_CONST,
    WasmFeatures::MULTI_MEMORY,
    WasmFeatures::CUSTOM_PAGE_SIZES,
    WasmFeatures::MEMORY64,
    WasmFeatures::WIDE_ARITHMETIC,
    WasmFeatures::SIMD,
    WasmFeatures::RELAXED_SIMD,
];

pub fn cmd_inspect(vfs: &Path, args: &InspectArgs) -> Result<()> {
    let (author_id, app_id) = get_id(vfs.to_path_buf(), args).context("get app ID")?;
    let rom_path = vfs.join("roms").join(&author_id).join(&app_id);
    if !rom_path.exists() {
        bail!("app {author_id}.{app_id} is not installed");
    }

    {
        let sizes = collect_sizes(&rom_path);
        print_sizes(sizes);
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
        print_wasm_stats(wasm_stats);
    }
    {
        let images_stats = inspect_images(&rom_path).context("inspect images")?;
        print_images_stats(images_stats);
    }
    {
        let audios_stats = inspect_audios(&rom_path).context("inspect audios")?;
        print_audios_stats(audios_stats);
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

struct ValErr {
    source: String,
    message: String,
}

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct Feature {
    name: String,
    supported: bool,
}

#[derive(Default)]
struct WasmStats {
    imports: Vec<(String, String)>,
    exports: Vec<String>,
    validation_errors: Vec<ValErr>,
    required_features: Vec<Feature>,
    memory: u64,
    memory_bytes: u64,
    globals: u32,
    functions: u32,
    code_size: u32,
    data_size: usize,
}

fn inspect_wasm(bin_path: &Path) -> anyhow::Result<WasmStats> {
    let parser = Parser::new(0);
    let mut stats = WasmStats::default();
    let input_bytes = std::fs::read(bin_path).context("read wasm binary")?;

    let mut validator = Validator::new_with_features(WasmFeatures::all());
    if let Err(err) = validator.validate_all(&input_bytes) {
        let err = ValErr {
            source: "module".to_owned(),
            message: format!("{err}"),
        };
        stats.validation_errors.push(err);
    } else {
        stats.required_features = get_required_features(&input_bytes);
    }

    let input = parser.parse_all(&input_bytes);
    let mut validator = Validator::new_with_features(WasmFeatures::all());
    for payload in input {
        let payload = payload?;
        if !matches!(payload, CodeSectionEntry(_))
            && let Err(err) = validator.payload(&payload)
        {
            let sname = get_section_name(&payload);
            let err = ValErr {
                source: format!("{sname} section"),
                message: format!("{err}"),
            };
            stats.validation_errors.push(err);
        }
        match payload {
            ImportSection(import_sections) => {
                for imports in import_sections {
                    let imports = imports?;
                    for import in imports {
                        let (_, import) = import?;
                        let name = (import.module.to_owned(), import.name.to_owned());
                        stats.imports.push(name);
                    }
                }
            }
            GlobalSection(globals) => {
                stats.globals = globals.count();
            }
            MemorySection(memories) => {
                for memory in memories {
                    let memory = memory?;
                    stats.memory += memory.initial;
                    let page_size = 2u64.pow(memory.page_size_log2.unwrap_or(16));
                    stats.memory_bytes += memory.initial * page_size;
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

/// Get the list of wasm features (specs) that must be supported to run the binary.
fn get_required_features(input_bytes: &[u8]) -> Vec<Feature> {
    let mut res = Vec::new();
    for (name, feature) in WasmFeatures::all().iter_names() {
        if requires_feature(input_bytes, feature) {
            let supported = SUPPORTED_FEATURES.contains(&feature);
            let name = name.to_ascii_lowercase().replace('_', " ");
            res.push(Feature { name, supported });
        }
    }
    res.sort_unstable();
    res
}

/// Check if the binary can be parsed with the given feature disabled.
fn requires_feature(input_bytes: &[u8], feature: WasmFeatures) -> bool {
    let mut features = WasmFeatures::all();
    features.remove(feature);
    let mut validator = Validator::new_with_features(features);
    validator.validate_all(input_bytes).is_err()
}

const fn get_section_name(payload: &Payload<'_>) -> &'static str {
    match payload {
        Version { .. } => "version",
        TypeSection(_) => "type",
        ImportSection(_) => "import",
        FunctionSection(_) => "function",
        TableSection(_) => "table",
        MemorySection(_) => "memory",
        TagSection(_) => "tag",
        GlobalSection(_) => "global",
        ExportSection(_) => "export",
        StartSection { .. } => "start",
        ElementSection(_) => "element",
        DataCountSection { .. } => "data_count",
        DataSection(_) => "data",
        CodeSectionStart { .. } => "code",
        CodeSectionEntry(..) => "code entry",
        ModuleSection { .. } => "module",
        InstanceSection(_) => "instance",
        CoreTypeSection(_) => "core_type",
        ComponentSection { .. } => "component",
        ComponentInstanceSection(_) => "component_instance",
        ComponentAliasSection(_) => "component_alias",
        ComponentTypeSection(_) => "component_type",
        ComponentCanonicalSection(_) => "component_canonical",
        ComponentStartSection { .. } => "component_start",
        ComponentImportSection(_) => "component_import",
        ComponentExportSection(_) => "component_export",
        CustomSection(_) => "custom",
        UnknownSection { .. } => "unknown",
        End(_) => "end",
        _ => "unsupported",
    }
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
        }
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

struct AudioStats {
    name: String,
    channels: u8,
    depth: u8,
    adpcm: bool,
    sample_rate: u16,
    duration: f32,
}

fn inspect_audios(rom_path: &Path) -> anyhow::Result<Vec<AudioStats>> {
    let dir = fs::read_dir(rom_path)?;
    let mut stats = Vec::new();
    for entry in dir {
        let entry = entry?;
        if let Some(stat) = inspect_audio(&entry.path()) {
            stats.push(stat);
        }
    }
    Ok(stats)
}

fn inspect_audio(path: &Path) -> Option<AudioStats> {
    let audio_bytes = fs::read(path).ok()?;
    if audio_bytes.len() < 4 {
        return None;
    }
    if audio_bytes[0] != 0x31 {
        return None;
    }
    let stereo = audio_bytes[1] & 0b_100 != 0;
    let channels: u8 = if stereo { 2 } else { 1 };
    let is16 = audio_bytes[1] & 0b_010 != 0;
    let depth: u8 = if is16 { 16 } else { 8 };
    let adpcm = audio_bytes[1] & 0b_001 != 0;
    let sample_rate = u16::from_le_bytes([audio_bytes[2], audio_bytes[3]]);

    let audio_bytes = &audio_bytes[4..];
    #[expect(clippy::cast_precision_loss)]
    let mut duration = audio_bytes.len() as f32 / f32::from(u16::from(channels) * sample_rate);
    if is16 {
        duration /= 2.0;
    }

    let name = path.file_name()?;
    let name: String = name.to_str()?.to_string();
    Some(AudioStats {
        name,
        channels,
        depth,
        adpcm,
        sample_rate,
        duration,
    })
}

fn print_meta(meta: &Meta<'_>) {
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

fn print_sizes(sizes: HashMap<OsString, u64>) {
    println!("{}", "files".blue());
    let width = sizes.keys().map(|n| n.len()).max().unwrap_or_default();
    for (name, size) in sizes {
        let name = name.to_str().unwrap_or("???");
        let size = format_size(size);
        println!("  {name:width$} {size}");
    }
    println!();
}

fn print_wasm_stats(stats: WasmStats) {
    let code_size = format_size(stats.code_size.into());
    let code_size = code_size.trim_start();
    let data_size = format_size(stats.data_size as u64);
    let data_size = data_size.trim_start();

    println!("{}", "wasm binary".blue());
    println!("  {}: {}", "code size".cyan(), code_size);
    println!("  {}: {}", "data size".cyan(), data_size);
    println!("  {}: {}", "functions".cyan(), stats.functions);
    println!("  {}:   {}", "globals".cyan(), stats.globals);
    let mem_size = format_size(stats.memory_bytes);
    println!(
        "  {}:    {} page{} ({})",
        "memory".cyan(),
        stats.memory,
        if stats.memory == 1 { "" } else { "s" },
        mem_size.trim(),
    );
    println!("  {}:   {}", "imports".cyan(), stats.imports.len());
    for (mod_name, func_name) in stats.imports {
        let mod_name = mod_name.magenta();
        println!("    {mod_name}.{func_name}");
    }
    println!("  {}:   {}", "exports".cyan(), stats.exports.len());
    for export in stats.exports {
        // TODO: when we stabilize the list of callbacks, highlight unknown exports.
        println!("    {export}");
    }

    let has_errors = !stats.validation_errors.is_empty();
    if has_errors {
        let n = stats.validation_errors.len();
        println!("  {}: {}", "validation errors".red(), n);
        for err in stats.validation_errors {
            println!("    {}: {}", err.source.magenta(), err.message);
        }
    } else {
        let n = stats.required_features.len();
        let max = WasmFeatures::all().iter().count();
        println!("  {}: {}/{}", "required features".cyan(), n, max);
        for feature in stats.required_features {
            let name = if feature.supported {
                format!("✅ {}", feature.name.green())
            } else {
                format!("❓ {}", feature.name.red())
            };
            println!("    {name}");
        }
    }
}

fn print_images_stats(stats: Vec<ImageStats>) {
    if stats.is_empty() {
        return;
    }
    println!();
    println!("{}", "images".blue());
    for stat in stats {
        print_image_stats(stat);
    }
}

fn print_image_stats(stats: ImageStats) {
    println!("  {}", stats.name.magenta());
    println!("    {}:    {}", "bpp".cyan(), stats.bpp);
    println!("    {}:  {}", "width".cyan(), stats.width);
    println!("    {}: {}", "height".cyan(), stats.height);
    println!("    {}: {}", "pixels".cyan(), stats.pixels);
    println!("    {}", "colors".cyan());
    for (i, swap) in stats.swaps.into_iter().enumerate() {
        if let Some(swap) = swap {
            let name = get_color_name(swap);
            let swap = swap + 1;
            println!("      {i:>2} -> {swap:>2}  {name}");
        } else {
            println!("      {i:>2} ->  0  transparent");
        }
    }
}

fn print_audios_stats(stats: Vec<AudioStats>) {
    if stats.is_empty() {
        return;
    }
    println!();
    println!("{}", "audio".blue());
    for stat in stats {
        print_audio_stats(stat);
    }
}

fn print_audio_stats(stats: AudioStats) {
    println!("  {}", stats.name.magenta());
    let mono = if stats.channels == 1 {
        "mono"
    } else {
        "stereo"
    };
    println!("    {}:    {} ({mono})", "channels".cyan(), stats.channels);
    println!("    {}:   {} bits", "bit depth".cyan(), stats.depth);
    println!("    {}: {}", "sample rate".cyan(), stats.sample_rate);
    println!("    {}:  {}", "compressed".cyan(), stats.adpcm);
    println!("    {}:    {:0.1}s", "duration".cyan(), stats.duration);
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
