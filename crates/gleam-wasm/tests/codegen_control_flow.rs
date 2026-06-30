use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::module::*;
use gleam_wasm::ir::{HEAP_TYPE_STRUCT, Instr, RefType, ValType};

#[test]
fn test_tail_call_codegen() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_function("factorial".into(), 0);

    let expr = TypedExpr::TailCall {
        name: "factorial".into(),
        args: vec![TypedExpr::Int(5)],
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(body[0], Instr::I64Const(5));
    assert_eq!(body[1], Instr::ReturnCall(0));
}

#[test]
fn test_tail_recursive_factorial_wat() {
    use gleam_wasm::codegen::ast::{BinOp, TypedExpr};
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);

    let func_type_idx = mapper.module_mut().add_func_type(FuncType {
        params: vec![ValType::I64, ValType::I64],
        results: vec![ValType::I64],
    });
    mapper.register_function("factorial_loop".into(), 0);
    mapper.register_local("n".into(), 0);
    mapper.register_local("acc".into(), 1);

    // factorial_loop(n, acc):
    //   if n <= 1 then acc else factorial_loop(n-1, n*acc)
    let body_expr = TypedExpr::If {
        cond: Box::new(TypedExpr::BinOp {
            op: BinOp::Le,
            left: Box::new(TypedExpr::Var {
                name: "n".into(),
                type_: ValType::I64,
            }),
            right: Box::new(TypedExpr::Int(1)),
            type_: ValType::I64,
        }),
        then_branch: Box::new(TypedExpr::Var {
            name: "acc".into(),
            type_: ValType::I64,
        }),
        else_branch: Box::new(TypedExpr::TailCall {
            name: "factorial_loop".into(),
            args: vec![
                TypedExpr::BinOp {
                    op: BinOp::Sub,
                    left: Box::new(TypedExpr::Var {
                        name: "n".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Int(1)),
                    type_: ValType::I64,
                },
                TypedExpr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(TypedExpr::Var {
                        name: "n".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Var {
                        name: "acc".into(),
                        type_: ValType::I64,
                    }),
                    type_: ValType::I64,
                },
            ],
            type_: ValType::I64,
        }),
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &body_expr);
    mapper.module_mut().functions.push(Function {
        name: Some("$factorial_loop".into()),
        type_index: func_type_idx,
        locals: vec![],
        body,
    });

    let wat = emit_wat(mapper.module());
    println!("{wat}");

    assert!(wat.contains("return_call $0"));
    assert!(wat.contains("i64.sub"));
    assert!(wat.contains("i64.mul"));
    assert!(wat.contains("i64.le_s"));
    assert!(wat.contains("if"));
}

#[test]
fn test_closure_codegen() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::Closure {
        params: vec![],
        captured: vec![],
        body: Box::new(TypedExpr::Int(42)),
        type_: ValType::I64,
        inner_func_idx: None,
    };

    let body = compile_expr(&mut mapper, &expr);
    assert!(matches!(body[0], Instr::StructNew(_)));
}

#[test]
fn test_struct_new_codegen() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_adt(
        "Option",
        &[("Some", vec![("value", ValType::I64)]), ("None_", vec![])],
    );

    let expr = TypedExpr::StructNew {
        type_index: 2, // $None_ variant
        fields: vec![],
        type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
    };

    let body = compile_expr(&mut mapper, &expr);
    assert_eq!(body[0], Instr::StructNew(2));
}
