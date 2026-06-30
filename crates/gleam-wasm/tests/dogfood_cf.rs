use gleam_wasm::codegen::ast::*;
/// End-to-end: Compile a Lustre counter update() function for Cloudflare Workers
/// and verify the output is valid Cloudflare-compatible WASM.
use gleam_wasm::codegen::*;
use gleam_wasm::emit::emit_wasm;
use gleam_wasm::ir::types::*;
use std::fs;

#[test]
fn test_cloudflare_lustre_counter_update() {
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "update".into(),
            params: vec![("state".into(), ValType::I64)],
            return_type: ValType::I64,
            // update(state, action) = state + 1 (simplified for CF target)
            body: TypedExpr::BinOp {
                op: BinOp::Add,
                left: Box::new(TypedExpr::Var {
                    name: "state".into(),
                    type_: ValType::I64,
                }),
                right: Box::new(TypedExpr::Int(1)),
                type_: ValType::I64,
            },
        }],
        exports: vec!["update".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    // Write files
    let out_dir = "dogfooding/lustre-counter/output";
    fs::create_dir_all(out_dir).unwrap();
    fs::write(format!("{out_dir}/counter_cf.wat"), &wat).unwrap();
    fs::write(format!("{out_dir}/counter_cf.wasm"), &wasm).unwrap();

    println!("CF Wasm: {} bytes", wasm.len());

    // Valid WASM
    assert!(&wasm[0..4] == b"\0asm");
    assert_eq!(wasm[4..8], [0x01, 0x00, 0x00, 0x00]);

    // No GC instructions
    assert!(!wat.contains("struct.new"));
    assert!(!wat.contains("ref.test"));
    assert!(!wat.contains("ref.cast"));
    assert!(!wat.contains("br_on_cast"));

    // Has linear memory
    assert!(wat.contains("memory"));

    // Has user function
    assert!(wat.contains("\"update\""));

    // Has runtime builtins
    assert!(wat.contains("$alloc"));
    assert!(wat.contains("$make_tagged"));
    assert!(wat.contains("$get_tag"));
    assert!(wat.contains("$get_payload"));

    // Size check: should be well under 2KB for a simple counter
    assert!(
        wasm.len() < 2048,
        "CF counter too large: {} bytes",
        wasm.len()
    );
}

#[test]
fn test_cloudflare_lustre_counter_with_match() {
    // Full Lustre counter update() with ADT match on Action
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "update".into(),
            params: vec![("state".into(), ValType::I64)],
            return_type: ValType::I64,
            // Simplified: state + 1 (ADT match codegen for linear target
            // will be added when full match lowering is implemented)
            body: TypedExpr::BinOp {
                op: BinOp::Add,
                left: Box::new(TypedExpr::Var {
                    name: "state".into(),
                    type_: ValType::I64,
                }),
                right: Box::new(TypedExpr::Int(1)),
                type_: ValType::I64,
            },
        }],
        exports: vec!["update".into()],
        imports: vec![],
        adt_types: vec![(
            "Action".into(),
            vec![("Incr".into(), vec![]), ("Decr".into(), vec![])],
        )],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    println!("CF Wasm with ADT tags: {} bytes", wasm.len());

    // Valid WASM
    assert!(&wasm[0..4] == b"\0asm");
    assert_eq!(wasm[4..8], [0x01, 0x00, 0x00, 0x00]);

    // No GC
    assert!(!wat.contains("struct.new"));

    // Has memory, allocator, user code
    assert!(wat.contains("memory"));
    assert!(wat.contains("$alloc"));
    assert!(wat.contains("\"update\""));
}
