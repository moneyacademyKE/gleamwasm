use crate::ir::module::*;

mod encoder;

use encoder::{encode_function_body, encode_instr, encode_sleb128, encode_u32, encode_valtype};

/// Emit binary WASM. For linear modules (no GC types), round-trips through
/// the wast crate's built-in binary encoder which produces spec-valid output.
/// Falls back to hand-rolled encoder for GC modules.
///
/// In-memory convenience wrapper over [`emit_wasm_to`] (GAP #5).
pub fn emit_wasm(module: &Module) -> Vec<u8> {
    encode_module(module)
}

/// Stream-oriented emitter (GAP #5): writes the binary module to any
/// `std::io::Write` sink — file, socket, or in-memory cursor.
pub fn emit_wasm_to(module: &Module, out: &mut dyn std::io::Write) -> std::io::Result<()> {
    out.write_all(&encode_module(module))
}

fn encode_module(module: &Module) -> Vec<u8> {
    let has_gc = module
        .types
        .iter()
        .any(|td| matches!(td.kind, TypeDefKind::Struct(..) | TypeDefKind::Array(..)));

    if !has_gc {
        let wat = crate::emit::emit_wat(module);
        #[allow(clippy::collapsible_if)]
        if let Ok(buf) = wast::parser::ParseBuffer::new(&wat) {
            if let Ok(mut mod_result) = wast::parser::parse::<wast::core::Module>(&buf) {
                if let Ok(wasm) = mod_result.encode() {
                    if &wasm[0..4] == b"\0asm" {
                        return wasm;
                    }
                }
            }
        }
    }

    encode_manual(module)
}

/// Hand-rolled binary encoder: the primary path for GC modules and the
/// fallback when the wast round-trip fails. Public for property-based
/// fuzzing (tests/fuzz.rs validates its output with wasmparser).
pub fn encode_manual(module: &Module) -> Vec<u8> {
    let mut buf = Vec::new();

    // Magic + version
    buf.extend_from_slice(b"\0asm");
    buf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

    // --- Type section (id=1) ---
    if !module.types.is_empty() {
        encode_section(&mut buf, 1, |buf| {
            encode_u32(buf, module.types.len() as u32);
            for td in &module.types {
                match &td.kind {
                    TypeDefKind::Func(ft) => {
                        buf.push(0x60); // functype
                        encode_u32(buf, ft.params.len() as u32);
                        for p in &ft.params {
                            encode_valtype(buf, p);
                        }
                        encode_u32(buf, ft.results.len() as u32);
                        for r in &ft.results {
                            encode_valtype(buf, r);
                        }
                    }
                    TypeDefKind::Struct(_st) => {
                        // Skip GC struct types in binary — they need wasm GC binary encoding
                        // which wast v59 doesn't support. Struct types are emitted in WAT only.
                        // For binary, we emit a placeholder functype.
                        buf.push(0x60); // functype placeholder
                        encode_u32(buf, 0);
                        encode_u32(buf, 0);
                    }
                    TypeDefKind::Array(_at) => {
                        buf.push(0x60); // functype placeholder
                        encode_u32(buf, 0);
                        encode_u32(buf, 0);
                    }
                }
            }
        });
    }

    // --- Import section (id=2) ---
    if !module.imports.is_empty() {
        encode_section(&mut buf, 2, |buf| {
            encode_u32(buf, module.imports.len() as u32);
            for imp in &module.imports {
                encode_name(buf, &imp.module);
                encode_name(buf, &imp.name);
                match &imp.desc {
                    ImportDesc::Func { type_index } => {
                        buf.push(0x00); // functype import
                        encode_u32(buf, *type_index);
                    }
                }
            }
        });
    }

    // --- Function section (id=3) ---
    if !module.functions.is_empty() {
        encode_section(&mut buf, 3, |buf| {
            encode_u32(buf, module.functions.len() as u32);
            for func in &module.functions {
                encode_u32(buf, func.type_index);
            }
        });
    }

    // --- Table section (id=4) ---
    if !module.tables.is_empty() {
        encode_section(&mut buf, 4, |buf| {
            encode_u32(buf, module.tables.len() as u32);
            for t in &module.tables {
                buf.push(match t.kind {
                    TableKind::FuncRef => 0x70,
                    TableKind::ExternRef => 0x6F,
                });
                if let Some(max) = t.max {
                    buf.push(0x01); // limits: max present
                    encode_u32(buf, t.min);
                    encode_u32(buf, max);
                } else {
                    buf.push(0x00); // limits: no max
                    encode_u32(buf, t.min);
                }
            }
        });
    }

    // --- Memory section (id=5) ---
    if !module.memories.is_empty() {
        encode_section(&mut buf, 5, |buf| {
            encode_u32(buf, 1); // 1 memory
            let mem = &module.memories[0];
            if let Some(max) = mem.max {
                buf.push(0x01); // with max
                encode_u32(buf, mem.min);
                encode_u32(buf, max);
            } else {
                buf.push(0x00); // no max
                encode_u32(buf, mem.min);
            }
        });
    }

    // --- Global section (id=6) ---
    if !module.globals.is_empty() {
        encode_section(&mut buf, 6, |buf| {
            encode_u32(buf, module.globals.len() as u32);
            for global in &module.globals {
                encode_valtype(buf, &global.type_.val_type);
                buf.push(if global.type_.mutable { 0x01 } else { 0x00 });
                for instr in &global.init {
                    encode_instr(buf, instr);
                }
                buf.push(0x0B); // end
            }
        });
    }

    // --- Export section (id=7) ---
    if !module.exports.is_empty() {
        encode_section(&mut buf, 7, |buf| {
            encode_u32(buf, module.exports.len() as u32);
            for exp in &module.exports {
                encode_name(buf, &exp.name);
                match exp.kind {
                    ExportKind::Func(idx) => {
                        buf.push(0x00); // func export
                        encode_u32(buf, idx);
                    }
                }
            }
        });
    }

    // --- Element section (id=9) ---
    if !module.elements.is_empty() {
        encode_section(&mut buf, 9, |buf| {
            encode_u32(buf, module.elements.len() as u32);
            for seg in &module.elements {
                encode_u32(buf, 0); // flags 0: active, table 0, vec(funcidx)
                buf.push(0x41); // i32.const (offset expr)
                encode_sleb128(buf, seg.offset as i64);
                buf.push(0x0b); // end
                encode_u32(buf, seg.items.len() as u32);
                for idx in &seg.items {
                    encode_u32(buf, *idx);
                }
            }
        });
    }

    // --- Code section (id=10) ---
    if !module.functions.is_empty() {
        encode_section(&mut buf, 10, |buf| {
            encode_u32(buf, module.functions.len() as u32);
            for func in &module.functions {
                let body = encode_function_body(func);
                encode_u32(buf, body.len() as u32);
                buf.extend_from_slice(&body);
            }
        });
    }

    // --- Data section (id=11) ---
    if !module.data.is_empty() {
        encode_section(&mut buf, 11, |buf| {
            encode_u32(buf, module.data.len() as u32);
            for seg in &module.data {
                encode_u32(buf, 0); // flags 0: active, memory 0
                buf.push(0x41); // i32.const (offset expr)
                encode_sleb128(buf, seg.offset as i64);
                buf.push(0x0b); // end
                encode_u32(buf, seg.bytes.len() as u32);
                buf.extend_from_slice(&seg.bytes);
            }
        });
    }

    buf
}

fn encode_section(buf: &mut Vec<u8>, id: u8, f: impl FnOnce(&mut Vec<u8>)) {
    buf.push(id);
    let mut section_buf = Vec::new();
    f(&mut section_buf);
    encode_u32(buf, section_buf.len() as u32);
    buf.extend_from_slice(&section_buf);
}

fn encode_name(buf: &mut Vec<u8>, name: &str) {
    let bytes = name.as_bytes();
    encode_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::types::*;

    #[test]
    fn test_emit_wasm_binary_magic() {
        let module = Module::new();
        let wasm = emit_wasm(&module);
        assert_eq!(&wasm[0..4], b"\0asm");
        assert_eq!(wasm[4..8], [0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_emit_wasm_simple_function() {
        let mut module = Module::new();
        let func_type = module.add_func_type(FuncType {
            params: vec![ValType::I64, ValType::I64],
            results: vec![ValType::I64],
        });
        module.functions.push(Function {
            name: None,
            type_index: func_type,
            locals: vec![
                Local::param("$a", ValType::I64),
                Local::param("$b", ValType::I64),
            ],
            body: vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::I64Add],
        });
        module.exports.push(Export {
            name: "add".into(),
            kind: ExportKind::Func(0),
        });

        let wasm = emit_wasm(&module);
        assert!(&wasm[0..4] == b"\0asm");
        assert!(wasm.len() > 30, "WASM too small: {}", wasm.len());
        // Should contain "add" export name
        let wasm_str = String::from_utf8_lossy(&wasm);
        assert!(wasm_str.contains("add"), "export name not found");
    }

    #[test]
    fn test_encode_leb128() {
        let mut buf = Vec::new();
        encode_u32(&mut buf, 0);
        assert_eq!(buf, [0x00]);

        buf.clear();
        encode_u32(&mut buf, 128);
        assert_eq!(buf, [0x80, 0x01]);

        buf.clear();
        encode_u32(&mut buf, 42);
        assert_eq!(buf, [0x2A]);
    }

    #[test]
    fn test_emit_wasm_to_matches_vec_api() {
        // GAP #5: the Write-sink variant must produce byte-identical output.
        let mut module = Module::new();
        let func_type = module.add_func_type(FuncType {
            params: vec![],
            results: vec![ValType::I64],
        });
        module.functions.push(Function {
            name: Some("$answer".into()),
            type_index: func_type,
            locals: vec![],
            body: vec![Instr::I64Const(42)],
        });
        module.exports.push(Export {
            name: "answer".into(),
            kind: ExportKind::Func(0),
        });

        let mut streamed: Vec<u8> = Vec::new();
        emit_wasm_to(&module, &mut streamed).expect("write to Vec cannot fail");
        assert_eq!(streamed, emit_wasm(&module));
        assert_eq!(&streamed[0..4], b"\0asm");
    }
}
