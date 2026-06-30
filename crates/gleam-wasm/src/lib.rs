pub mod cli;
pub mod codegen;
pub mod emit;
pub mod ffi;
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
            build_module(&input, &output, target, emit_wat)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    WasmWeb,
    WasmWasi,
    WasmCf,
}

fn build_module(input: &str, output: &str, target: Target, emit_wat: bool) -> Result<(), String> {
    use crate::codegen::{compile_module_with_opt, compile_to_linear, GleamModule};
    use crate::emit::emit_wasm;

    let module_def = if input.ends_with(".gleam") {
        let source = std::fs::read_to_string(input)
            .map_err(|e| format!("failed to read {input}: {e}"))?;
        crate::parse::parse_module(&source)
            .map_err(|e| format!("parse error at line {} col {}: {}", e.line, e.col, e.message))?
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
