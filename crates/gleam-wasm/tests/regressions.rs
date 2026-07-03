use gleam_wasm::codegen::{GleamFunctionDef, GleamModule, TypedExpr, compile_module_with_opt};
use gleam_wasm::ir::types::{HEAP_TYPE_NONE, RefType, ValType};

#[test]
fn parser_accepts_normal_list_literals() {
    let source = "pub fn values() {\n  [1, 2, 3]\n}\n";
    let module = gleam_wasm::parse::parse_module(source).expect("list literal should parse");
    assert_eq!(module.functions.len(), 1);
    assert!(matches!(
        module.functions[0].body,
        TypedExpr::ListCons { .. }
    ));
}

#[test]
fn parser_sequences_multiple_let_statements() {
    let source = "pub fn pipeline() {\n  let a = 1\n  let b = a + 2\n  b + 3\n}\n";
    let module = gleam_wasm::parse::parse_module(source).expect("multiple lets should parse");
    assert_eq!(module.functions.len(), 1);
    assert!(matches!(module.functions[0].body, TypedExpr::Let { .. }));
}

#[test]
fn parser_skips_generic_return_type_annotations() {
    let source = "pub fn range(start: Int, end: Int) -> List(Int) {\n  [start, end]\n}\n";
    let module = gleam_wasm::parse::parse_module(source).expect("generic return type should parse");
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "range");
}

#[test]
fn manual_encoder_emits_single_byte_for_nullref_fallback_locals() {
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "nil".into(),
            params: vec![],
            return_type: ValType::Ref(RefType::RefNull(HEAP_TYPE_NONE)),
            body: TypedExpr::Let {
                name: "x".into(),
                value: Box::new(TypedExpr::Nil),
                body: Box::new(TypedExpr::Var {
                    name: "x".into(),
                    type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_NONE)),
                }),
                type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_NONE)),
            },
        }],
        exports: vec!["nil".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let output = compile_module_with_opt(&module_def, gleam_wasm::Target::WasmWeb);
    let wasm = output.wasm.expect("wasm output");
    let nullref_result_type = [0x60, 0x00, 0x01, 0x71];
    assert!(
        wasm.windows(nullref_result_type.len())
            .any(|w| w == nullref_result_type)
    );
    assert!(!wasm.windows(5).any(|w| w == [0x60, 0x00, 0x01, 0x70, 0x71]));
}

#[test]
fn compile_module_declares_shadowed_let_locals() {
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "shadow".into(),
            params: vec![("x".into(), ValType::I64)],
            return_type: ValType::I64,
            body: TypedExpr::Let {
                name: "x".into(),
                value: Box::new(TypedExpr::Int(1)),
                body: Box::new(TypedExpr::Let {
                    name: "x".into(),
                    value: Box::new(TypedExpr::Int(2)),
                    body: Box::new(TypedExpr::Var {
                        name: "x".into(),
                        type_: ValType::I64,
                    }),
                    type_: ValType::I64,
                }),
                type_: ValType::I64,
            },
        }],
        exports: vec!["shadow".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let output = compile_module_with_opt(&module_def, gleam_wasm::Target::WasmWeb);
    assert!(output.wat.contains("local.set 1"), "{}", output.wat);
    assert!(output.wat.contains("local.set 2"), "{}", output.wat);
    assert!(output.wat.contains("local.get 2"), "{}", output.wat);
}
