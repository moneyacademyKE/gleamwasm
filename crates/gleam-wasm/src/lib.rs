pub mod cli;
pub mod codegen;
pub mod emit;
pub mod ffi;
pub mod frontend;
pub mod ir;
pub mod parse;
pub mod validate;
pub mod wasm_opt;

use clap::Parser as _;

pub fn run() -> Result<(), String> {
    let args = cli::Args::parse();
    match args.command {
        cli::Command::Build {
            target,
            frontend,
            input,
            output,
            emit_wat,
        } => {
            let target = match target.as_deref() {
                Some("wasm-web") | None => crate::Target::WasmWeb,
                Some("wasm-wasi") => crate::Target::WasmWasi,
                Some("wasm-cf") => crate::Target::WasmCf,
                Some(t) => return Err(format!("unknown target: {t}")),
            };
            let frontend = match frontend.as_str() {
                "sidecar" => Frontend::Sidecar,
                "builtin" => Frontend::Builtin,
                other => return Err(format!("unknown frontend: {other}")),
            };
            build_module(&input, &output, target, emit_wat, frontend)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    WasmWeb,
    WasmWasi,
    WasmCf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frontend {
    /// A2 sidecar (ADR-001): gleam-core inference behind a JSON data
    /// boundary — real types, real Gleam semantics (MVP subset).
    Sidecar,
    /// Legacy zero-dep recursive-descent parser: I64-only subset.
    Builtin,
}

fn build_module(
    input: &str,
    output: &str,
    target: Target,
    emit_wat: bool,
    frontend: Frontend,
) -> Result<(), String> {
    use crate::codegen::{compile_module_with_opt, compile_to_linear};
    use crate::emit::emit_wasm;

    let module_def = if input.ends_with(".gleam") {
        match frontend {
            Frontend::Sidecar => {
                let json = crate::frontend::run_sidecar(input)?;
                crate::frontend::module_from_json(&json)?
            }
            Frontend::Builtin => {
                let source = std::fs::read_to_string(input)
                    .map_err(|e| format!("failed to read {input}: {e}"))?;
                crate::parse::parse_module(&source).map_err(|e| {
                    format!(
                        "parse error at line {} col {}: {}",
                        e.line, e.col, e.message
                    )
                })?
            }
        }
    } else if input.ends_with(".json") {
        // Two-step UX (ADR-001 consequence 3): a pre-extracted typed AST.
        let json =
            std::fs::read_to_string(input).map_err(|e| format!("failed to read {input}: {e}"))?;
        crate::frontend::module_from_json(&json)?
    } else {
        // Default to empty module for test/development paths
        crate::codegen::GleamModule {
            functions: vec![],
            exports: vec![],
            imports: vec![],
            adt_types: vec![],
        }
    };

    if target == Target::WasmCf {
        let (cf_module, wat) = compile_to_linear(&module_def);
        if emit_wat {
            let wat_path = format!("{output}.wat");
            std::fs::write(&wat_path, &wat)
                .map_err(|e| format!("failed to write WAT to {wat_path}: {e}"))?;
            eprintln!("WAT written to {wat_path}");
        }
        let wasm_bytes = emit_wasm(&cf_module);
        std::fs::write(output, &wasm_bytes)
            .map_err(|e| format!("failed to write WASM to {output}: {e}"))?;
        eprintln!(
            "WASM (Cloudflare-compatible linear memory) written to {output} ({size} bytes)",
            size = wasm_bytes.len()
        );
        return Ok(());
    }

    let compile_output = compile_module_with_opt(&module_def, target);

    if emit_wat {
        let wat_path = format!("{output}.wat");
        std::fs::write(&wat_path, &compile_output.wat)
            .map_err(|e| format!("failed to write WAT to {wat_path}: {e}"))?;
        eprintln!("WAT written to {wat_path}");
    }

    if let Some(wasm_bytes) = &compile_output.wasm {
        std::fs::write(output, wasm_bytes)
            .map_err(|e| format!("failed to write WASM to {output}: {e}"))?;
        eprintln!(
            "WASM written to {output} ({size} bytes)",
            size = wasm_bytes.len()
        );
    } else {
        return Err("no binary WASM produced (WAT→binary encode failed)".into());
    }

    Ok(())
}
