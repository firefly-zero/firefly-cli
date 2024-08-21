use crate::args::InspectArgs;
use crate::file_names::BIN;
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use std::path::Path;
use wasmparser::Parser;
use wasmparser::Payload::*;

pub fn cmd_inspect(vfs: &Path, args: &InspectArgs) -> Result<()> {
    let id = match &args.id {
        Some(id) => id.to_owned(),
        None => detect_id()?,
    };
    let Some((author_id, app_id)) = id.split_once('.') else {
        bail!("invalid app id: dot not found");
    };
    let rom_path = vfs.join("roms").join(author_id).join(app_id);
    if !rom_path.exists() {
        bail!("app {id} is not installed");
    }
    let bin_path = rom_path.join(BIN);
    let wasm_stats = inspect_wasm(&bin_path)?;
    print_wasm_stats(&wasm_stats);
    Ok(())
}

fn detect_id() -> Result<String> {
    todo!()
}

#[derive(Default)]
struct WasmStats {
    imports: Vec<(String, String)>,
    exports: Vec<String>,
    memory: u64,
    globals: u32,
    functions: u32,
    code_size: u32,
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

fn print_wasm_stats(stats: &WasmStats) {
    println!("{}", "wasm binary:".blue());
    println!("  {}: {}", "code size".cyan(), stats.code_size);
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
        println!("    {export}");
    }
}
