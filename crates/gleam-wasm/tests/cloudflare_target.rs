/// Tests for the Cloudflare-compatible linear memory target.
/// Verifies: no GC instructions, valid WASM magic, memory export.
use gleam_wasm::codegen::compile_to_linear;
use gleam_wasm::codegen::{GleamFunctionDef, GleamModule, TypedExpr};
use gleam_wasm::emit::emit_wasm;
use gleam_wasm::ir::{Instr, ValType};

#[test]
fn test_cf_target_produces_valid_wasm() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    // Valid WASM magic
    assert!(&wasm[0..4] == b"\0asm", "invalid magic");
    assert_eq!(wasm[4..8], [0x01, 0x00, 0x00, 0x00], "invalid version");

    // Must have memory section (id=5)
    assert!(
        wasm.windows(2).any(|w| w[0] == 0x05),
        "missing memory section"
    );

    // No GC instructions in WAT
    assert!(!wat.contains("struct.new"));
    assert!(!wat.contains("ref.test"));
    assert!(!wat.contains("ref.cast"));
    assert!(!wat.contains("br_on_cast"));
    assert!(!wat.contains("(sub"));

    println!("CF-compatible WASM: {} bytes", wasm.len());
}

#[test]
fn test_cf_target_has_bump_allocator() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (_module, wat) = compile_to_linear(&module_def);

    // Verify builtin runtime functions
    assert!(wat.contains("$alloc"));
    assert!(wat.contains("$make_tagged"));
    assert!(wat.contains("$get_tag"));
    assert!(wat.contains("$get_payload"));
    assert!(wat.contains("memory"));
    assert!(wat.contains("(memory"));
}

#[test]
fn test_cf_target_exports_memory() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (_module, wat) = compile_to_linear(&module_def);
    assert!(wat.contains("(export \"memory\""));
}

#[test]
fn test_cf_target_size_bound() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    // Runtime + allocator should be under 1KB
    assert!(
        wasm.len() < 1024,
        "CF runtime too large: {} bytes (target < 1024)",
        wasm.len()
    );
}

#[test]
fn test_cf_target_exports_user_function_indices() {
    let module_def = GleamModule {
        functions: vec![
            GleamFunctionDef {
                name: "first".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::Int(1),
            },
            GleamFunctionDef {
                name: "second".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::Int(2),
            },
        ],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let first_index = module
        .functions
        .iter()
        .position(|func| func.name.as_deref() == Some("first"))
        .unwrap() as u32;
    let second_index = module
        .functions
        .iter()
        .position(|func| func.name.as_deref() == Some("second"))
        .unwrap() as u32;
    assert_eq!(module.exports[0].name, "first");
    assert_eq!(
        module.exports[0].kind,
        gleam_wasm::ir::module::ExportKind::Func(first_index)
    );
    assert_eq!(module.exports[1].name, "second");
    assert_eq!(
        module.exports[1].kind,
        gleam_wasm::ir::module::ExportKind::Func(second_index)
    );
}

#[test]
fn test_cf_tail_call_uses_named_function_index() {
    let module_def = GleamModule {
        functions: vec![
            GleamFunctionDef {
                name: "first".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::Int(1),
            },
            GleamFunctionDef {
                name: "second".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::TailCall {
                    name: "first".into(),
                    args: vec![],
                    type_: ValType::I64,
                },
            },
        ],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let first_index = module
        .functions
        .iter()
        .position(|func| func.name.as_deref() == Some("first"))
        .unwrap() as u32;
    let second_index = module
        .functions
        .iter()
        .position(|func| func.name.as_deref() == Some("second"))
        .unwrap();
    assert!(
        module.functions[second_index]
            .body
            .contains(&Instr::ReturnCall(first_index))
    );
}

#[test]
fn test_cf_normal_call_uses_named_function_index() {
    let module_def = GleamModule {
        functions: vec![
            GleamFunctionDef {
                name: "first".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::Int(1),
            },
            GleamFunctionDef {
                name: "second".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::Call {
                    name: "first".into(),
                    args: vec![],
                    type_: ValType::I64,
                },
            },
        ],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let first_index = module
        .functions
        .iter()
        .position(|func| func.name.as_deref() == Some("first"))
        .unwrap() as u32;
    let second_index = module
        .functions
        .iter()
        .position(|func| func.name.as_deref() == Some("second"))
        .unwrap();
    assert!(
        module.functions[second_index]
            .body
            .contains(&Instr::Call(first_index))
    );
    assert!(
        !module.functions[second_index]
            .body
            .contains(&Instr::Unreachable)
    );
}
