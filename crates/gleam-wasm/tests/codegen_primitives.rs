use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::module::*;
use gleam_wasm::ir::{HEAP_TYPE_NONE, Instr, RefType, ValType};

#[test]
fn test_compile_int_addition() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let func_type_idx = mapper.module_mut().add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I64],
    });
    mapper.register_function("add_two_numbers".into(), 0);

    let expr = TypedExpr::Call {
        name: "add_two_numbers".into(),
        args: vec![TypedExpr::Int(10), TypedExpr::Int(20)],
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);
    mapper.module_mut().functions.push(Function {
        name: Some("$test".into()),
        type_index: func_type_idx,
        locals: vec![],
        body,
    });

    let wat = emit_wat(mapper.module());
    println!("{wat}");
    assert!(wat.contains("i64.const 10"));
    assert!(wat.contains("i64.const 20"));
    assert!(wat.contains("call $0"));
}

#[test]
fn test_compile_binary_ops() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::BinOp {
        op: BinOp::Mul,
        left: Box::new(TypedExpr::Int(6)),
        right: Box::new(TypedExpr::Int(7)),
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(body.len(), 3);
    assert_eq!(body[0], Instr::I64Const(6));
    assert_eq!(body[1], Instr::I64Const(7));
    assert_eq!(body[2], Instr::I64Mul);
}

#[test]
fn test_compile_float_ops() {
    use gleam_wasm::codegen::ast::{BinOp, TypedExpr};
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::BinOp {
        op: BinOp::Add,
        left: Box::new(TypedExpr::Float(2.71)),
        right: Box::new(TypedExpr::Float(2.86)),
        type_: ValType::F64,
    };

    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(body[0], Instr::F64Const(2.71));
    assert_eq!(body[1], Instr::F64Const(2.86));
    assert_eq!(body[2], Instr::F64Add);
}

#[test]
fn test_compile_bool() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::Bool(true);
    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(body[0], Instr::I32Const(1));
}

#[test]
fn test_compile_nil() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::Nil;
    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(
        body[0],
        Instr::RefNull(ValType::Ref(RefType::RefNull(HEAP_TYPE_NONE)))
    );
}

#[test]
fn test_compile_if_then_else() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::If {
        cond: Box::new(TypedExpr::Bool(true)),
        then_branch: Box::new(TypedExpr::Int(1)),
        else_branch: Box::new(TypedExpr::Int(0)),
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);
    let wat_line = format!("{}", body[1]);
    assert!(wat_line.contains("i64.const 1"));
    assert!(wat_line.contains("else"));
    assert!(wat_line.contains("i64.const 0"));
}

#[test]
fn test_compile_let_binding() {
    use gleam_wasm::codegen::ast::{BinOp, TypedExpr};
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::Let {
        name: "x".into(),
        value: Box::new(TypedExpr::Int(42)),
        body: Box::new(TypedExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(TypedExpr::Var {
                name: "x".into(),
                type_: ValType::I64,
            }),
            right: Box::new(TypedExpr::Int(8)),
            type_: ValType::I64,
        }),
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(body[0], Instr::I64Const(42));
    assert_eq!(body[1], Instr::LocalSet(0));
    assert_eq!(body[2], Instr::LocalGet(0));
    assert_eq!(body[3], Instr::I64Const(8));
    assert_eq!(body[4], Instr::I64Add);
}
