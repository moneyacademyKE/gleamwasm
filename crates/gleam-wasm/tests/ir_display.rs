use gleam_wasm::ir::types::{HeapType, RefType};
use gleam_wasm::ir::{Block, Instr, Local, LocalKind, ValType};

#[test]
fn test_instr_display_i64() {
    assert_eq!(Instr::I64Const(42).to_string(), "i64.const 42");
    assert_eq!(Instr::I64Add.to_string(), "i64.add");
    assert_eq!(Instr::I64Sub.to_string(), "i64.sub");
    assert_eq!(Instr::I64Mul.to_string(), "i64.mul");
    assert_eq!(Instr::I64DivS.to_string(), "i64.div_s");
}

#[test]
fn test_instr_display_f64() {
    assert_eq!(Instr::F64Const(2.71).to_string(), "f64.const 2.71");
    assert_eq!(Instr::F64Add.to_string(), "f64.add");
    assert_eq!(Instr::F64Mul.to_string(), "f64.mul");
    assert_eq!(Instr::F64Div.to_string(), "f64.div");
}

#[test]
fn test_instr_display_gc_ops() {
    assert_eq!(Instr::StructNew(0).to_string(), "struct.new $0");
    assert_eq!(
        Instr::StructGet {
            type_index: 1,
            field_index: 0
        }
        .to_string(),
        "struct.get $1 $f0"
    );
}

#[test]
fn test_instr_display_control_flow() {
    assert_eq!(Instr::Return.to_string(), "return");
    assert_eq!(Instr::Call(3).to_string(), "call $3");
    assert_eq!(Instr::CallRef(2).to_string(), "call_ref $2");
    assert_eq!(Instr::ReturnCall(1).to_string(), "return_call $1");
    assert_eq!(Instr::ReturnCallRef(0).to_string(), "return_call_ref $0");
    assert_eq!(Instr::Drop.to_string(), "drop");
}

#[test]
fn test_instr_display_cast() {
    let src = ValType::Ref(RefType::RefNull(HeapType::TypeIndex(0)));
    let dst = ValType::Ref(RefType::Ref(HeapType::TypeIndex(1)));
    assert_eq!(
        Instr::BrOnCast {
            label: 0,
            src: src.clone(),
            dst: dst.clone()
        }
        .to_string(),
        "br_on_cast 0 (ref null $0) (ref $1)"
    );
}

#[test]
fn test_local_constructors() {
    let param = Local::param("$a", ValType::I64);
    let var = Local::var("$temp", ValType::F64);

    assert!(matches!(param.kind, LocalKind::Param(ValType::I64)));
    assert!(matches!(var.kind, LocalKind::Var(ValType::F64)));
    assert_eq!(param.name.as_deref(), Some("$a"));
}

#[test]
fn test_block_builder() {
    let block = Block::new(
        vec![Instr::I64Const(1), Instr::I64Const(2), Instr::I64Add],
        Some(ValType::I64),
    )
    .label("result");

    let output = Instr::Block(Box::new(block)).to_string();
    println!("{output}");
    assert!(output.contains("block $result (result i64)"));
    assert!(output.contains("i64.const 1"));
    assert!(output.contains("i64.add"));
    assert!(output.contains("end"));
}
