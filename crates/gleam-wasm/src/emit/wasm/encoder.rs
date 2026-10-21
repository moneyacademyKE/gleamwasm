//! Instruction + LEB128 encoders: Instr -> bytes, ValType -> bytes.
//!
//! Pure byte emitters — emission order is load-bearing (LEB128 bug history,
//! see audit.md). Move whole functions only; never reorder output.

use crate::ir::module::*;
use crate::ir::types::*;

pub(super) fn encode_u32(buf: &mut Vec<u8>, mut n: u32) {
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

pub(super) fn encode_valtype(buf: &mut Vec<u8>, vt: &ValType) {
    match vt {
        ValType::I32 => buf.push(0x7F),
        ValType::I64 => buf.push(0x7E),
        ValType::F32 => buf.push(0x7D),
        ValType::F64 => buf.push(0x7C),
        ValType::Ref(RefType::RefNull(HeapType::Abstract(AbstractHeapType::Extern))) => {
            buf.push(0x6F); // externref
        }
        ValType::Ref(RefType::RefNull(HeapType::Abstract(AbstractHeapType::None_))) => {
            buf.push(0x71); // nullref fallback
        }
        ValType::Ref(_) => {
            buf.push(0x6F); // externref fallback
        }
    }
}

pub(super) fn encode_function_body(func: &Function) -> Vec<u8> {
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

pub(super) fn encode_instr(buf: &mut Vec<u8>, instr: &Instr) {
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
        Instr::CallIndirect {
            type_index,
            table_index,
        } => {
            buf.push(0x11);
            encode_u32(buf, *type_index);
            encode_u32(buf, *table_index);
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

pub(super) fn encode_sleb128(buf: &mut Vec<u8>, v: i64) {
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
