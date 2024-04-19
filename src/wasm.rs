use std::path::Path;
use wasm_encoder::{Component, ComponentSectionId, Encode, Module, Section};
use wasmparser::Payload::*;
use wasmparser::*;

/// Remove custom sections from the given wasm file.
///
/// The custom sections contain DWARF debug info, info about the code producer, etc.
/// We don't use any of that in the runtime.
///
/// https://github.com/bytecodealliance/wasm-tools/blob/main/src/bin/wasm-tools/strip.rs
pub(crate) fn strip_custom(bin_path: &Path) -> anyhow::Result<()> {
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
                let mut parent = match stack.pop() {
                    Some(c) => c,
                    None => break,
                };
                if output.starts_with(&Component::HEADER) {
                    parent.push(ComponentSectionId::Component as u8);
                    output.encode(&mut parent);
                } else {
                    parent.push(ComponentSectionId::CoreModule as u8);
                    output.encode(&mut parent);
                }
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
