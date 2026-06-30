use gleam_wasm::codegen::*;
use gleam_wasm::ir::{HEAP_TYPE_STRUCT, RefType, ValType};

fn build_match_test(
    mapper: &mut TypeMapper,
    scrutinee_name: &str,
    expr: &TypedExpr,
) -> Vec<String> {
    mapper.register_local(scrutinee_name.into(), 0);
    let body = compile_expr(mapper, expr);
    body.iter().map(|i| i.to_string()).collect()
}

#[test]
fn test_compile_match_two_variant_adt() {
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_adt(
        "Option",
        &[("Some", vec![("value", ValType::I64)]), ("None_", vec![])],
    );

    let expr = TypedExpr::Match {
        scrutinee: Box::new(TypedExpr::Var {
            name: "opt".into(),
            type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
        }),
        cases: vec![
            MatchCase {
                variant_index: 1,
                bindings: vec!["val".into()],
                body: Box::new(TypedExpr::Var {
                    name: "val".into(),
                    type_: ValType::I64,
                }),
            },
            MatchCase {
                variant_index: 2,
                bindings: vec![],
                body: Box::new(TypedExpr::Int(0)),
            },
        ],
        type_: ValType::I64,
    };

    let lines = build_match_test(&mut mapper, "opt", &expr);
    let joined = lines.join("\n");
    println!("{joined}");

    assert!(
        joined.contains("ref.test"),
        "expected ref.test for dispatch"
    );
    assert!(
        joined.contains("struct.get $1 $f0"),
        "expected struct.get for Some field"
    );
}

#[test]
fn test_compile_shape_match() {
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_adt(
        "Shape",
        &[
            ("Circle", vec![("radius", ValType::F64)]),
            (
                "Rect",
                vec![("width", ValType::F64), ("height", ValType::F64)],
            ),
        ],
    );

    let expr = TypedExpr::Match {
        scrutinee: Box::new(TypedExpr::Var {
            name: "shape".into(),
            type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
        }),
        cases: vec![
            MatchCase {
                variant_index: 1,
                bindings: vec!["r".into()],
                body: Box::new(TypedExpr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(TypedExpr::Var {
                        name: "r".into(),
                        type_: ValType::F64,
                    }),
                    right: Box::new(TypedExpr::Float(2.0)),
                    type_: ValType::F64,
                }),
            },
            MatchCase {
                variant_index: 2,
                bindings: vec!["w".into(), "h".into()],
                body: Box::new(TypedExpr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(TypedExpr::Var {
                        name: "w".into(),
                        type_: ValType::F64,
                    }),
                    right: Box::new(TypedExpr::Var {
                        name: "h".into(),
                        type_: ValType::F64,
                    }),
                    type_: ValType::F64,
                }),
            },
        ],
        type_: ValType::F64,
    };

    let lines = build_match_test(&mut mapper, "shape", &expr);
    let joined = lines.join("\n");
    println!("{joined}");

    assert!(joined.contains("ref.test"));
    assert!(joined.contains("struct.get $1 $f0"), "Circle radius field");
    assert!(joined.contains("struct.get $2 $f0"), "Rect width field");
    assert!(joined.contains("struct.get $2 $f1"), "Rect height field");
}

#[test]
fn test_compile_list_cons_deconstruction() {
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_adt(
        "List",
        &[
            (
                "Cons",
                vec![
                    ("head", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
                    ("tail", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
                ],
            ),
            ("Nil", vec![]),
        ],
    );

    let expr = TypedExpr::Match {
        scrutinee: Box::new(TypedExpr::Var {
            name: "list".into(),
            type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
        }),
        cases: vec![
            MatchCase {
                variant_index: 1,
                bindings: vec!["hd".into(), "tl".into()],
                body: Box::new(TypedExpr::Var {
                    name: "hd".into(),
                    type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
                }),
            },
            MatchCase {
                variant_index: 2,
                bindings: vec![],
                body: Box::new(TypedExpr::Nil),
            },
        ],
        type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
    };

    let lines = build_match_test(&mut mapper, "list", &expr);
    let joined = lines.join("\n");
    println!("{joined}");

    assert!(joined.contains("ref.test"));
    assert!(joined.contains("struct.get $1 $f0"), "Cons head");
    assert!(joined.contains("struct.get $1 $f1"), "Cons tail");
}
