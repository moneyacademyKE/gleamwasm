use gleam_wasm::codegen::ast::*;
/// Self-contained GleamUnison WASM: all 12 FFI stubs in pure WASM, zero JS imports.
use gleam_wasm::codegen::*;
use gleam_wasm::emit::emit_wasm;
use gleam_wasm::ir::types::*;
use gleam_wasm::validate::validate_module;
use std::fs;

#[test]
fn test_self_contained_has_zero_imports() {
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

    let adapter = compile_self_contained(&module_def);
    let wasm = emit_wasm(&adapter.module);

    assert!(&wasm[0..4] == b"\0asm");
    // Zero imports — all functions are in the module
    assert!(!adapter.wat.contains("(import"), "should have no imports");

    // All 17 builtins present
    assert!(adapter.wat.contains("$alloc"));
    assert!(adapter.wat.contains("$make_tagged"));
    assert!(adapter.wat.contains("$get_tag"));
    assert!(adapter.wat.contains("$get_payload"));
    assert!(adapter.wat.contains("$hash_bytes"));
    assert!(adapter.wat.contains("$hex_to_bytes"));
    assert!(adapter.wat.contains("$hash_equal"));
    assert!(adapter.wat.contains("$hash_to_hex"));
    assert!(adapter.wat.contains("$state_get"));
    assert!(adapter.wat.contains("$state_set"));
    assert!(adapter.wat.contains("$file_read"));
    assert!(adapter.wat.contains("$file_write"));
    assert!(adapter.wat.contains("$log"));
    assert!(adapter.wat.contains("$now_ms"));
    assert!(adapter.wat.contains("$timestamp"));
    assert!(adapter.wat.contains("$eval"));
    assert!(adapter.wat.contains("$memcpy"));

    // User function exported
    assert!(adapter.wat.contains("\"identity\""));
    assert!(adapter.wat.contains("memory"));

    validate_module(&adapter.module, true).expect("SC adapter validates");

    println!("Self-contained WASM: {} bytes, 0 imports", wasm.len());
}

#[test]
fn test_self_contained_hash_function() {
    // Verifies the FNV-1a hash is embedded (not imported)
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "hash_wrapper".into(),
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
        }],
        exports: vec!["hash_wrapper".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let adapter = compile_self_contained(&module_def);
    let wasm = emit_wasm(&adapter.module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(!adapter.wat.contains("(import"));
    assert!(adapter.wat.contains("$hash_bytes"));
    assert!(adapter.wat.contains("i32.load8_u")); // FNV-1a uses byte loads
    assert!(adapter.wat.contains("i32.xor")); // hash xors

    validate_module(&adapter.module, true).expect("SC validates");

    println!("Self-contained hash: {} bytes", wasm.len());
}

#[test]
fn test_self_contained_state_kv_store() {
    // Verifies state_get/set are embedded with linear memory hash table
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "state_wrapper".into(),
            params: vec![("val".into(), ValType::I64)],
            return_type: ValType::I64,
            body: TypedExpr::BinOp {
                op: BinOp::Add,
                left: Box::new(TypedExpr::Var {
                    name: "val".into(),
                    type_: ValType::I64,
                }),
                right: Box::new(TypedExpr::Int(1)),
                type_: ValType::I64,
            },
        }],
        exports: vec!["state_wrapper".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let adapter = compile_self_contained(&module_def);
    let wasm = emit_wasm(&adapter.module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(!adapter.wat.contains("(import"));
    assert!(adapter.wat.contains("$state_get"));
    assert!(adapter.wat.contains("$state_set"));
    assert!(adapter.wat.contains("$memcpy"));

    validate_module(&adapter.module, true).expect("SC validates");

    println!("Self-contained KV store: {} bytes", wasm.len());
}

#[test]
fn test_self_contained_full_deploy() {
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
            GleamFunctionDef {
                name: "state_demo".into(),
                params: vec![("val".into(), ValType::I64)],
                return_type: ValType::I64,
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

    let adapter = compile_self_contained(&module_def);
    let wasm = emit_wasm(&adapter.module);

    validate_module(&adapter.module, true).expect("SC full deploy validates");

    let deploy_dir = "dogfooding/gleamunison/cf-deploy";
    fs::create_dir_all(deploy_dir).unwrap();
    fs::write(format!("{deploy_dir}/gleamunison_sc.wasm"), &wasm).unwrap();
    fs::write(format!("{deploy_dir}/gleamunison_sc.wat"), &adapter.wat).unwrap();

    assert!(&wasm[0..4] == b"\0asm");
    assert!(!adapter.wat.contains("(import"), "self-contained must have zero imports");
    assert!(adapter.wat.contains("\"local_var_index\""));
    assert!(adapter.wat.contains("\"range\""));
    assert!(adapter.wat.contains("\"hash\""));
    assert!(adapter.wat.contains("\"level1\""));
    assert!(adapter.wat.contains("\"state_demo\""));
    assert!(adapter.wat.contains("$hash_bytes"));
    assert!(adapter.wat.contains("$state_get"));
    assert!(adapter.wat.contains("$state_set"));
    assert!(adapter.wat.contains("$memcpy"));
    assert!(adapter.wat.contains("memory"));
    assert!(!adapter.wat.contains("struct.new"));

    assert!(
        wasm.len() < 8192,
        "Self-contained WASM too large: {} bytes",
        wasm.len()
    );

    println!(
        "Self-contained GleamUnison WASM: {} bytes, 0 imports, 17 builtins",
        wasm.len()
    );
    println!("Deployed to {deploy_dir}");
}
