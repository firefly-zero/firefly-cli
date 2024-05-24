use anyhow::{bail, Context};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use wasm_encoder::{Component, ComponentSectionId, Encode, Module, Section};
use wasmparser::Payload::{ComponentSection, CustomSection, End, ModuleSection, Version};
use wasmparser::{Encoding, Parser};

/// Remove custom sections from the given wasm file.
///
/// The custom sections contain DWARF debug info, info about the code producer, etc.
/// We don't use any of that in the runtime.
///
/// Based on [wasm-strip].
///
/// [wasm-strip]: https://github.com/bytecodealliance/wasm-tools/blob/main/src/bin/wasm-tools/strip.rs
pub fn strip_custom(bin_path: &Path) -> anyhow::Result<()> {
    let parser = Parser::new(0);
    let input_bytes = std::fs::read(bin_path)?;
    let input = parser.parse_all(&input_bytes);
    let mut output = Vec::new();
    let mut stack = Vec::new();
    for payload in input {
        let payload = payload?;
        match payload {
            Version { encoding, .. } => {
                output.extend_from_slice(match encoding {
                    Encoding::Component => &Component::HEADER,
                    Encoding::Module => &Module::HEADER,
                });
            }
            ModuleSection { .. } | ComponentSection { .. } => {
                stack.push(std::mem::take(&mut output));
                continue;
            }
            End { .. } => {
                let Some(mut parent) = stack.pop() else { break };
                if output.starts_with(&Component::HEADER) {
                    parent.push(ComponentSectionId::Component as u8);
                } else {
                    parent.push(ComponentSectionId::CoreModule as u8);
                }
                output.encode(&mut parent);
                output = parent;
            }
            _ => {}
        }

        if let CustomSection(_) = &payload {
            continue;
        }
        if let Some((id, range)) = payload.as_section() {
            wasm_encoder::RawSection {
                id,
                data: &input_bytes[range],
            }
            .append_to(&mut output);
        }
    }
    std::fs::write(bin_path, output)?;
    Ok(())
}

/// Run [wasm-opt] on the given wasm binary.
///
/// [wasm-opt]: https://github.com/WebAssembly/binaryen
pub fn optimize(bin_path: &Path) -> anyhow::Result<()> {
    let Some(bin_path) = bin_path.to_str() else {
        return Ok(());
    };

    let output = Command::new("wasm-opt").arg("--version").output();
    if output.is_err() {
        return Ok(());
    }

    let output = Command::new("wasm-opt")
        .args(["-Oz", "--all-features", "-o", bin_path, bin_path])
        .output()
        .context("run wasm-opt")?;
    if !output.status.success() {
        std::io::stdout().write_all(&output.stdout)?;
        std::io::stderr().write_all(&output.stderr)?;
        let code = output.status.code().unwrap_or(1);
        bail!("subprocess exited with status code {code}");
    }
    Ok(())
}
