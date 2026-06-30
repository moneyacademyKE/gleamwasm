use gleam_wasm::codegen::ast::*;
/// End-to-end: GleamUnison CF Adapter — compiles GleamUnison functions
/// with Cloudflare import stubs for hash, state, file, and eval FFI.
use gleam_wasm::codegen::*;
use gleam_wasm::emit::emit_wasm;
use gleam_wasm::ir::types::*;
use gleam_wasm::validate::validate_module;
use std::fs;

#[test]
fn test_gleamunison_adapter_has_imports() {
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "hash_test".into(),
            params: vec![("ptr".into(), ValType::I64), ("len".into(), ValType::I64)],
            return_type: ValType::I64,
            body: TypedExpr::BinOp {
                op: BinOp::Add,
                left: Box::new(TypedExpr::Var {
                    name: "ptr".into(),
                    type_: ValType::I64,
                }),
                right: Box::new(TypedExpr::Var {
                    name: "len".into(),
                    type_: ValType::I64,
                }),
                type_: ValType::I64,
            },
        }],
        exports: vec!["hash_test".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let adapter = compile_gleamunison(&module_def);
    let wasm = emit_wasm(&adapter.module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(adapter.import_count == 12, "expected 12 imports, got {}", adapter.import_count);

    // WAT must contain import declarations
    assert!(adapter.wat.contains("(import \"gleamunison\" \"hash_bytes\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"hex_to_bytes\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"hash_equal\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"hash_to_hex\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"state_get\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"state_set\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"file_read\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"file_write\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"log\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"now_ms\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"timestamp\""));
    assert!(adapter.wat.contains("(import \"gleamunison\" \"eval\""));

    assert!(adapter.wat.contains("memory"));
    assert!(adapter.wat.contains("\"hash_test\""));
    assert!(adapter.wat.contains("$hash_bytes"));
    assert!(adapter.wat.contains("$hex_to_bytes"));

    validate_module(&adapter.module, true).expect("adapter validates");
}

#[test]
fn test_gleamunison_adapter_eval_stub() {
    // eval(expr_ptr: i32, expr_len: i32) — import 11, calls through to $hash_bytes runtime wrapper
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "eval_wrapper".into(),
            params: vec![
                ("expr_ptr".into(), ValType::I64),
                ("expr_len".into(), ValType::I64),
            ],
            return_type: ValType::I64,
            // Simple identity — returns expr_ptr (just to test the adapter compiles)
            body: TypedExpr::Var {
                name: "expr_ptr".into(),
                type_: ValType::I64,
            },
        }],
        exports: vec!["eval_wrapper".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let adapter = compile_gleamunison(&module_def);
    let wasm = emit_wasm(&adapter.module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(adapter.wat.contains("(import \"gleamunison\" \"eval\""));
    assert!(adapter.wat.contains("\"eval_wrapper\""));

    validate_module(&adapter.module, true).expect("adapter validates");
    println!("GleamUnison eval stub: {} bytes", wasm.len());
}

#[test]
fn test_gleamunison_adapter_full_deploy() {
    let module_def = GleamModule {
        functions: vec![
            GleamFunctionDef {
                name: "local_var_index".into(),
                params: vec![("lv".into(), ValType::I64)],
                return_type: ValType::I64,
                body: TypedExpr::Var {
                    name: "lv".into(),
                    type_: ValType::I64,
                },
            },
            GleamFunctionDef {
                name: "range".into(),
                params: vec![
                    ("start".into(), ValType::I64),
                    ("end".into(), ValType::I64),
                ],
                return_type: ValType::I64,
                body: TypedExpr::If {
                    cond: Box::new(TypedExpr::BinOp {
                        op: BinOp::Gt,
                        left: Box::new(TypedExpr::Var {
                            name: "start".into(),
                            type_: ValType::I64,
                        }),
                        right: Box::new(TypedExpr::Var {
                            name: "end".into(),
                            type_: ValType::I64,
                        }),
                        type_: ValType::I64,
                    }),
                    then_branch: Box::new(TypedExpr::Int(0)),
                    else_branch: Box::new(TypedExpr::Var {
                        name: "start".into(),
                        type_: ValType::I64,
                    }),
                    type_: ValType::I64,
                },
            },
            GleamFunctionDef {
                name: "hash".into(),
                params: vec![("n".into(), ValType::I64)],
                return_type: ValType::I64,
                body: TypedExpr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(TypedExpr::Var {
                        name: "n".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Int(16_777_619)),
                    type_: ValType::I64,
                },
            },
            GleamFunctionDef {
                name: "level1".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::If {
                    cond: Box::new(TypedExpr::BinOp {
                        op: BinOp::Lt,
                        left: Box::new(TypedExpr::Int(1)),
                        right: Box::new(TypedExpr::Int(2)),
                        type_: ValType::I64,
                    }),
                    then_branch: Box::new(TypedExpr::Int(100)),
                    else_branch: Box::new(TypedExpr::Int(0)),
                    type_: ValType::I64,
                },
            },
            // State get/set demo — stores then retrieves a value
            GleamFunctionDef {
                name: "state_demo".into(),
                params: vec![("val".into(), ValType::I64)],
                return_type: ValType::I64,
                // val + 1 (simulated state mutation — real impl would call state_set)
                body: TypedExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(TypedExpr::Var {
                        name: "val".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Int(1)),
                    type_: ValType::I64,
                },
            },
        ],
        exports: vec![
            "local_var_index".into(),
            "range".into(),
            "hash".into(),
            "level1".into(),
            "state_demo".into(),
        ],
        imports: vec![],
        adt_types: vec![],
    };

    let adapter = compile_gleamunison(&module_def);
    let wasm = emit_wasm(&adapter.module);

    validate_module(&adapter.module, true).expect("GleamUnison CF adapter validates");

    let deploy_dir = "dogfooding/gleamunison/cf-deploy";
    fs::create_dir_all(deploy_dir).unwrap();
    fs::write(format!("{deploy_dir}/gleamunison_cf.wasm"), &wasm).unwrap();
    fs::write(format!("{deploy_dir}/gleamunison_cf.wat"), &adapter.wat).unwrap();

    assert!(&wasm[0..4] == b"\0asm");
    assert_eq!(adapter.import_count, 12);
    assert!(adapter.wat.contains("(import \"gleamunison\" \"hash_bytes\""));
    assert!(adapter.wat.contains("\"local_var_index\""));
    assert!(adapter.wat.contains("\"range\""));
    assert!(adapter.wat.contains("\"hash\""));
    assert!(adapter.wat.contains("\"level1\""));
    assert!(adapter.wat.contains("\"state_demo\""));
    assert!(!adapter.wat.contains("struct.new"));

    assert!(
        wasm.len() < 4096,
        "GleamUnison CF adapter too large: {} bytes",
        wasm.len()
    );

    println!(
        "GleamUnison CF adapter: {} bytes, {} imports",
        wasm.len(),
        adapter.import_count
    );
    println!("Deployed to {deploy_dir}");
}
