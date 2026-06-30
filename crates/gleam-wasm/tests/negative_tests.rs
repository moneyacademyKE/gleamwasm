#[test]
#[should_panic(expected = "local not found")]
fn test_missing_local_panics() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;
    use gleam_wasm::ir::ValType;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::Var {
        name: "does_not_exist".into(),
        type_: ValType::I64,
    };

    let _ = compile_expr(&mut mapper, &expr);
}

#[test]
#[should_panic(expected = "function not found")]
fn test_missing_function_panics() {
    use gleam_wasm::codegen::ast::TypedExpr;
    use gleam_wasm::codegen::*;
    use gleam_wasm::ir::ValType;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    let expr = TypedExpr::Call {
        name: "missing_func".into(),
        args: vec![],
        type_: ValType::I64,
    };

    let _ = compile_expr(&mut mapper, &expr);
}

#[test]
fn test_compile_function_with_return() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::*;
    use gleam_wasm::ir::ValType;

    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "identity".into(),
            params: vec![("x".into(), ValType::I64)],
            return_type: ValType::I64,
            body: TypedExpr::Var {
                name: "x".into(),
                type_: ValType::I64,
            },
        }],
        exports: vec!["identity".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let wat = compile_module(&module_def, Target::WasmWeb, true);
    println!("{wat}");
    assert!(wat.contains("\"identity\""));
    assert!(wat.contains("(export"));
}
