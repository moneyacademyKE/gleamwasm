use super::types::{Block, Instr};
use std::fmt;

pub type Function = super::module::Function;
pub type Expr = super::module::Expr;

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::LocalGet(i) => write!(f, "local.get {i}"),
            Instr::LocalSet(i) => write!(f, "local.set {i}"),
            Instr::LocalTee(i) => write!(f, "local.tee {i}"),
            Instr::I32Const(v) => write!(f, "i32.const {v}"),
            Instr::I64Const(v) => write!(f, "i64.const {v}"),
            Instr::F64Const(v) => write!(f, "f64.const {v}"),
            Instr::RefNull(t) => write!(f, "ref.null {t}"),
            Instr::RefI31 => write!(f, "ref.i31"),
            Instr::I32Add => write!(f, "i32.add"),
            Instr::I32Sub => write!(f, "i32.sub"),
            Instr::I32Mul => write!(f, "i32.mul"),
            Instr::I32DivS => write!(f, "i32.div_s"),
            Instr::I32RemS => write!(f, "i32.rem_s"),
            Instr::I32Eq => write!(f, "i32.eq"),
            Instr::I32Ne => write!(f, "i32.ne"),
            Instr::I32LtS => write!(f, "i32.lt_s"),
            Instr::I32GtS => write!(f, "i32.gt_s"),
            Instr::I32LeS => write!(f, "i32.le_s"),
            Instr::I32GeS => write!(f, "i32.ge_s"),
            Instr::I64Add => write!(f, "i64.add"),
            Instr::I64Sub => write!(f, "i64.sub"),
            Instr::I64Mul => write!(f, "i64.mul"),
            Instr::I64DivS => write!(f, "i64.div_s"),
            Instr::I64RemS => write!(f, "i64.rem_s"),
            Instr::I64Eq => write!(f, "i64.eq"),
            Instr::I64Ne => write!(f, "i64.ne"),
            Instr::I64LtS => write!(f, "i64.lt_s"),
            Instr::I64GtS => write!(f, "i64.gt_s"),
            Instr::I64LeS => write!(f, "i64.le_s"),
            Instr::I64GeS => write!(f, "i64.ge_s"),
            Instr::F64Add => write!(f, "f64.add"),
            Instr::F64Sub => write!(f, "f64.sub"),
            Instr::F64Mul => write!(f, "f64.mul"),
            Instr::F64Div => write!(f, "f64.div"),
            Instr::F64Eq => write!(f, "f64.eq"),
            Instr::F64Ne => write!(f, "f64.ne"),
            Instr::F64Lt => write!(f, "f64.lt"),
            Instr::F64Gt => write!(f, "f64.gt"),
            Instr::F64Le => write!(f, "f64.le"),
            Instr::F64Ge => write!(f, "f64.ge"),
            Instr::StructNew(i) => write!(f, "struct.new ${i}"),
            Instr::StructGet {
                type_index,
                field_index,
            } => write!(f, "struct.get ${type_index} $f{field_index}"),
            Instr::StructSet {
                type_index,
                field_index,
            } => write!(f, "struct.set ${type_index} $f{field_index}"),
            Instr::ArrayNew(i) => write!(f, "array.new ${i}"),
            Instr::ArrayGet(i) => write!(f, "array.get ${i}"),
            Instr::ArraySet(i) => write!(f, "array.set ${i}"),
            Instr::ArrayLen => write!(f, "array.len"),
            Instr::RefTest(t) => write!(f, "ref.test {t}"),
            Instr::RefCast(t) => write!(f, "ref.cast {t}"),
            Instr::BrOnCast { label, src, dst } => write!(f, "br_on_cast {label} {src} {dst}"),
            Instr::BrOnCastFail { label, src, dst } => {
                write!(f, "br_on_cast_fail {label} {src} {dst}")
            }
            Instr::Block(b) => write_instr_block(f, "block", b),
            Instr::Loop(b) => write_instr_block(f, "loop", b),
            Instr::If {
                then_branch,
                else_branch,
            } => {
                let result = then_branch
                    .result
                    .as_ref()
                    .map(|t| format!(" (result {})", t.as_ref()))
                    .unwrap_or_default();
                if let Some(else_blk) = else_branch {
                    writeln!(f, "if{result}")?;
                    write_instr_block(f, "then", then_branch)?;
                    writeln!(f)?;
                    write_instr_block(f, "else", else_blk)?;
                    writeln!(f, "\nend")
                } else {
                    writeln!(f, "if{result}")?;
                    write_instr_block(f, "then", then_branch)?;
                    writeln!(f, "\nend")
                }
            }
            Instr::Br(i) => write!(f, "br {i}"),
            Instr::BrIf(i) => write!(f, "br_if {i}"),
            Instr::BrTable { branches, default } => {
                write!(f, "br_table ")?;
                for b in branches {
                    write!(f, "{b} ")?;
                }
                write!(f, "{default}")
            }
            Instr::Return => write!(f, "return"),
            Instr::Unreachable => write!(f, "unreachable"),
            Instr::Call(i) => write!(f, "call ${i}"),
            Instr::CallRef(i) => write!(f, "call_ref ${i}"),
            Instr::ReturnCall(i) => write!(f, "return_call ${i}"),
            Instr::ReturnCallRef(i) => write!(f, "return_call_ref ${i}"),
            Instr::RefFunc(i) => write!(f, "ref.func {i}"),
            Instr::Throw(i) => write!(f, "throw ${i}"),
            Instr::ExternConvertAny => write!(f, "extern.convert_any"),
            Instr::AnyConvertExtern => write!(f, "any.convert_extern"),
            Instr::Drop => write!(f, "drop"),
            Instr::Select => write!(f, "select"),
            Instr::GlobalGet(i) => write!(f, "global.get {i}"),
            Instr::GlobalSet(i) => write!(f, "global.set {i}"),
            Instr::I64ExtendI32U => write!(f, "i64.extend_i32_u"),
            Instr::I32WrapI64 => write!(f, "i32.wrap_i64"),
            Instr::I64Shl => write!(f, "i64.shl"),
            Instr::I64ShrU => write!(f, "i64.shr_u"),
            Instr::I64Or => write!(f, "i64.or"),
            Instr::I32And => write!(f, "i32.and"),
            Instr::I32Or => write!(f, "i32.or"),
            Instr::I32Xor => write!(f, "i32.xor"),
            Instr::I32Shl => write!(f, "i32.shl"),
            Instr::I32ShrS => write!(f, "i32.shr_s"),
            Instr::I32ShrU => write!(f, "i32.shr_u"),
            Instr::I32Load8U { offset, align } => {
                write!(f, "i32.load8_u offset={offset} align={align}")
            }
            Instr::I32Store8 { offset, align } => {
                write!(f, "i32.store8 offset={offset} align={align}")
            }
            Instr::I32Load { offset, align } => write!(f, "i32.load offset={offset} align={align}"),
            Instr::I32Store { offset, align } => {
                write!(f, "i32.store offset={offset} align={align}")
            }
            Instr::I64Load { offset, align } => {
                write!(f, "i64.load offset={offset} align={align}")
            }
            Instr::I64Store { offset, align } => {
                write!(f, "i64.store offset={offset} align={align}")
            }
            Instr::F64Load { offset, align } => {
                write!(f, "f64.load offset={offset} align={align}")
            }
            Instr::F64Store { offset, align } => {
                write!(f, "f64.store offset={offset} align={align}")
            }
            Instr::MemoryGrow => write!(f, "memory.grow"),
            Instr::MemorySize => write!(f, "memory.size"),
            Instr::F64ConvertSI32 => write!(f, "f64.convert_i32_s"),
            Instr::F64Trunc => write!(f, "f64.trunc"),
            Instr::I32TruncSatF64S => write!(f, "i32.trunc_sat_f64_s"),
        }
    }
}

fn write_instr_block(f: &mut fmt::Formatter<'_>, tag: &str, block: &Block) -> fmt::Result {
    if let Some(ref label) = block.label {
        write!(f, "{tag} ${label}")?;
    } else {
        write!(f, "{tag}")?;
    }
    if let Some(ref result) = block.result {
        write!(f, " (result {})", result.as_ref())?;
    }
    for instr in &block.instrs {
        write!(f, "\n  {instr}")?;
    }
    write!(f, "\nend")
}
