use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::types::*;

#[test]
fn test_list_map_module() {
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);

    mapper.register_adt(
        "List",
        &[
            (
                "Cons",
                vec![
                    ("head", ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY))),
                    ("tail", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
                ],
            ),
            ("Nil", vec![]),
        ],
    );

    let module = mapper.into_module();
    let wat = emit_wat(&module);
    println!("{wat}");

    assert!(wat.contains("(type $List (sub"));
    assert!(wat.contains("(type $Cons (sub $List)"));
    assert!(wat.contains("(type $Nil (sub $List)"));
    assert!(wat.contains("(field $head (ref null any))"));
    assert!(wat.contains("(field $tail (ref null struct))"));
}

#[test]
fn test_result_ok_err_codegen() {
    use gleam_wasm::codegen::ast::{MatchCase, TypedExpr};
    use gleam_wasm::codegen::*;

    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);

    mapper.register_adt(
        "Result",
        &[
            ("Ok", vec![("value", ValType::I64)]),
            ("Err", vec![("error", ValType::I64)]),
        ],
    );

    let scrutinee_type = ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT));
    mapper.register_local("result".into(), 0);

    let expr = TypedExpr::Match {
        scrutinee: Box::new(TypedExpr::Var {
            name: "result".into(),
            type_: scrutinee_type,
        }),
        cases: vec![
            MatchCase {
                variant_index: 1,
                bindings: vec!["ok_val".into()],
                body: Box::new(TypedExpr::TailCall {
                    name: "handle_success".into(),
                    args: vec![TypedExpr::Var {
                        name: "ok_val".into(),
                        type_: ValType::I64,
                    }],
                    type_: ValType::I64,
                }),
            },
            MatchCase {
                variant_index: 2,
                bindings: vec!["err_val".into()],
                body: Box::new(TypedExpr::Call {
                    name: "handle_error".into(),
                    args: vec![TypedExpr::Var {
                        name: "err_val".into(),
                        type_: ValType::I64,
                    }],
                    type_: ValType::I64,
                }),
            },
        ],
        type_: ValType::I64,
    };

    mapper.register_function("handle_success".into(), 0);
    mapper.register_function("handle_error".into(), 1);

    let body = compile_expr(&mut mapper, &expr);
    let output: Vec<String> = body.iter().map(|i| i.to_string()).collect();
    let joined = output.join("\n");

    assert!(joined.contains("ref.test"));
    assert!(joined.contains("struct.get $1 $f0"), "Ok field");
    assert!(joined.contains("struct.get $2 $f0"), "Err field");
    assert!(
        joined.contains("return_call $0"),
        "tail call handle_success"
    );
    assert!(joined.contains("call $1"), "call handle_error");
}
