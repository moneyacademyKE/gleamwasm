use crate::ir::module::*;
use crate::ir::types::*;

/// Emit binary WASM. For linear modules (no GC types), round-trips through
/// the wast crate's built-in binary encoder which produces spec-valid output.
/// Falls back to hand-rolled encoder for GC modules.
pub fn emit_wasm(module: &Module) -> Vec<u8> {
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

fn encode_manual(module: &Module) -> Vec<u8> {
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

    buf
}

fn encode_section(buf: &mut Vec<u8>, id: u8, f: impl FnOnce(&mut Vec<u8>)) {
    buf.push(id);
    let mut section_buf = Vec::new();
    f(&mut section_buf);
    encode_u32(buf, section_buf.len() as u32);
    buf.extend_from_slice(&section_buf);
}

fn encode_u32(buf: &mut Vec<u8>, mut n: u32) {
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if n == 0 {
            break;
        }
    }
}

fn encode_name(buf: &mut Vec<u8>, name: &str) {
    let bytes = name.as_bytes();
    encode_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

fn encode_valtype(buf: &mut Vec<u8>, vt: &ValType) {
    match vt {
        ValType::I32 => buf.push(0x7F),
        ValType::I64 => buf.push(0x7E),
        ValType::F32 => buf.push(0x7D),
        ValType::F64 => buf.push(0x7C),
        ValType::Ref(RefType::RefNull(HeapType::Abstract(AbstractHeapType::Extern))) => {
            buf.push(0x6F); // externref
        }
        ValType::Ref(RefType::RefNull(HeapType::Abstract(AbstractHeapType::None_))) => {
            buf.push(0x70); // anyref → fallback
            buf.push(0x71); // nullref → fallback
        }
        ValType::Ref(_) => {
            buf.push(0x6F); // externref fallback
        }
    }
}

fn encode_function_body(func: &Function) -> Vec<u8> {
    let mut body = Vec::new();

    // Locals: count non-param locals, grouped by type
    let vars: Vec<&crate::ir::types::Local> = func
        .locals
        .iter()
        .filter(|l| matches!(l.kind, LocalKind::Var(_)))
        .collect();

    if vars.is_empty() {
        encode_u32(&mut body, 0);
    } else {
        // Group consecutive same-type locals
        let mut groups: Vec<(u32, u8)> = Vec::new();
        for v in &vars {
            let enc_type = match v.kind.val_type() {
                ValType::I32 => 0x7Fu8,
                ValType::I64 => 0x7Eu8,
                ValType::F32 => 0x7Du8,
                ValType::F64 => 0x7Cu8,
                _ => 0x7Eu8, // default to i64
            };
            if let Some(last) = groups.last_mut()
                && last.1 == enc_type
            {
                last.0 += 1;
                continue;
            }
            groups.push((1, enc_type));
        }
        encode_u32(&mut body, groups.len() as u32);
        for (count, enc_type) in &groups {
            encode_u32(&mut body, *count);
            body.push(*enc_type);
        }
    }

    // Instructions
    for instr in &func.body {
        encode_instr(&mut body, instr);
    }

    // End opcode (0x0B) required at end of function body
    body.push(0x0B);

    body
}

fn encode_instr(buf: &mut Vec<u8>, instr: &Instr) {
    match instr {
        Instr::LocalGet(i) => {
            buf.push(0x20);
            encode_u32(buf, *i);
        }
        Instr::LocalSet(i) => {
            buf.push(0x21);
            encode_u32(buf, *i);
        }
        Instr::I64Const(v) => {
            buf.push(0x42);
            encode_i64(buf, *v);
        }
        Instr::I32Const(v) => {
            buf.push(0x41);
            encode_i32(buf, *v);
        }
        Instr::F64Const(v) => {
            buf.push(0x44);
            buf.extend_from_slice(&v.to_le_bytes());
        }
        Instr::I64Add => buf.push(0x7C),
        Instr::I64Sub => buf.push(0x7D),
        Instr::I64Mul => buf.push(0x7E),
        Instr::I64DivS => buf.push(0x7F),
        Instr::I64Eq => buf.push(0x51),
        Instr::I64Ne => buf.push(0x52),
        Instr::I64LtS => buf.push(0x53),
        Instr::I64GtS => buf.push(0x55),
        Instr::I64LeS => buf.push(0x57),
        Instr::I64GeS => buf.push(0x59),
        Instr::I32Add => buf.push(0x6A),
        Instr::I32Sub => buf.push(0x6B),
        Instr::I32Mul => buf.push(0x6C),
        Instr::I32DivS => buf.push(0x6D),
        Instr::I32Eq => buf.push(0x46),
        Instr::I32Ne => buf.push(0x47),
        Instr::I32LtS => buf.push(0x48),
        Instr::I32GtS => buf.push(0x4A),
        Instr::I32LeS => buf.push(0x4C),
        Instr::I32GeS => buf.push(0x4E),
        Instr::LocalTee(i) => {
            buf.push(0x22);
            encode_u32(buf, *i);
        }
        Instr::F64Add => buf.push(0xA0),
        Instr::F64Sub => buf.push(0xA1),
        Instr::F64Mul => buf.push(0xA2),
        Instr::F64Div => buf.push(0xA3),
        Instr::F64Eq => buf.push(0x61),
        Instr::F64Ne => buf.push(0x62),
        Instr::F64Lt => buf.push(0x63),
        Instr::F64Gt => buf.push(0x64),
        Instr::F64Le => buf.push(0x65),
        Instr::F64Ge => buf.push(0x66),
        Instr::Call(i) => {
            buf.push(0x10);
            encode_u32(buf, *i);
        }
        Instr::ReturnCall(i) => {
            buf.push(0x12);
            encode_u32(buf, *i);
        }
        Instr::RefFunc(i) => {
            buf.push(0xD2);
            encode_u32(buf, *i);
        }
        Instr::Throw(i) => {
            buf.push(0xFB);
            encode_u32(buf, 8); // throw_ref
            encode_u32(buf, *i);
        }
        Instr::Return => buf.push(0x0F),
        Instr::Drop => buf.push(0x1A),
        Instr::Br(i) => {
            buf.push(0x0C);
            encode_u32(buf, *i);
        }
        Instr::BrIf(i) => {
            buf.push(0x0D);
            encode_u32(buf, *i);
        }
        Instr::Unreachable => buf.push(0x00),
        Instr::If {
            then_branch,
            else_branch,
        } => {
            buf.push(0x04); // if
            // One-armed if must have empty block type per WASM spec
            if else_branch.is_some() {
                if let Some(ref result) = then_branch.result {
                    encode_valtype(buf, result);
                } else {
                    buf.push(0x40);
                }
            } else {
                buf.push(0x40); // empty
            }
            for i in &then_branch.instrs {
                encode_instr(buf, i);
            }
            if let Some(else_blk) = else_branch {
                buf.push(0x05); // else
                for i in &else_blk.instrs {
                    encode_instr(buf, i);
                }
            }
            buf.push(0x0B); // end
        }
        Instr::RefNull(_) => {
            buf.push(0xD0);
            buf.push(0x71);
        } // ref.null none
        Instr::StructNew(i) => {
            buf.push(0xFB);
            encode_u32(buf, 0);
            encode_u32(buf, *i);
        }
        Instr::StructGet {
            type_index,
            field_index,
        } => {
            buf.push(0xFB);
            encode_u32(buf, 2);
            encode_u32(buf, *type_index);
            encode_u32(buf, *field_index);
        }
        Instr::Block(b) => {
            buf.push(0x02); // block
            if let Some(ref result) = b.result {
                encode_valtype(buf, result);
            } else {
                buf.push(0x40);
            }
            for i in &b.instrs {
                encode_instr(buf, i);
            }
            buf.push(0x0B); // end
        }
        Instr::I64ExtendI32U => buf.push(0xAD),
        Instr::I32WrapI64 => buf.push(0xA7),
        Instr::I64Shl => buf.push(0x86),
        Instr::I32Load8U { offset, align } => {
            buf.push(0x2D);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::I32Store8 { offset, align } => {
            buf.push(0x3A);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::I32Load { offset, align } => {
            buf.push(0x28);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::I32Store { offset, align } => {
            buf.push(0x36);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::F64Load { offset, align } => {
            buf.push(0x2B);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::F64Store { offset, align } => {
            buf.push(0x39);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::I64Load { offset, align } => {
            buf.push(0x29);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::I64Store { offset, align } => {
            buf.push(0x37);
            encode_u32(buf, *align);
            encode_u32(buf, *offset);
        }
        Instr::I32And => buf.push(0x71),
        Instr::I32Or => buf.push(0x72),
        Instr::I32Xor => buf.push(0x73),
        Instr::I32Shl => buf.push(0x74),
        Instr::I32ShrS => buf.push(0x75),
        Instr::I32ShrU => buf.push(0x76),
        Instr::Select => buf.push(0x1B),
        Instr::MemoryGrow => {
            buf.push(0x40);
            buf.push(0x00);
        }
        Instr::MemorySize => {
            buf.push(0x3F);
            buf.push(0x00);
        }
        Instr::GlobalGet(i) => {
            buf.push(0x23);
            encode_u32(buf, *i);
        }
        Instr::GlobalSet(i) => {
            buf.push(0x24);
            encode_u32(buf, *i);
        }
        Instr::F64ConvertSI32 => buf.push(0xB7),
        Instr::F64Trunc => buf.push(0x9D),
        Instr::I32TruncSatF64S => {
            buf.push(0xFC);
            encode_u32(buf, 4);
        }
        _ => {
            buf.push(0x00);
        } // unreachable fallback
    }
}

fn encode_i32(buf: &mut Vec<u8>, v: i32) {
    encode_sleb128(buf, v as i64);
}

fn encode_sleb128(buf: &mut Vec<u8>, v: i64) {
    let mut n = v;
    loop {
        let mut byte = (n & 0x7f) as u8;
        n >>= 7;
        if (n == 0 && (byte & 0x40) == 0) || (n == -1 && (byte & 0x40) != 0) {
            buf.push(byte);
            break;
        }
        byte |= 0x80;
        buf.push(byte);
    }
}

fn encode_i64(buf: &mut Vec<u8>, v: i64) {
    let mut n = v as u64;
    loop {
        let mut byte = (n & 0x7f) as u8;
        n >>= 7;
        if (n == 0 && (byte & 0x40) == 0) || (n == u64::MAX && (byte & 0x40) != 0) {
            buf.push(byte);
            break;
        }
        byte |= 0x80;
        buf.push(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
