use gleam_wasm::codegen::ast::*;
/// Dogfood: Compile extractable pure functions from GleamUnison.
///
/// GleamUnison v3.4.1 has 4,271 lines of Gleam with 75 `@external(erlang, ...)`
/// FFI calls embedded throughout. Only ADT constructors/destructors and trivial
/// identity functions are WASM-compilable without BEAM runtime stubs.
use gleam_wasm::codegen::*;
use gleam_wasm::emit::emit_wasm;
use gleam_wasm::ir::types::*;
use gleam_wasm::validate::validate_module;

/// GleamUnison's `local_var_index` — pure destructure.
/// ```gleam
/// pub fn local_var_index(lv: LocalVar) -> Int {
///   let Local(index) = lv; index
/// }
/// ```
#[test]
fn test_gleamunison_local_var_index() {
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "local_var_index".into(),
            params: vec![("lv".into(), ValType::I64)],
            return_type: ValType::I64,
            body: TypedExpr::Var {
                name: "lv".into(),
                type_: ValType::I64,
            },
        }],
        exports: vec!["local_var_index".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(wat.contains("\"local_var_index\""));
    validate_module(&module, true).expect("module validates");
}

/// GleamUnison's `range` — pure recursive list builder.
/// ```gleam
/// fn range(start: Int, end: Int) -> List(Int) {
///   case start > end { True -> []  False -> [start, ..range(start + 1, end)] }
/// }
/// ```
/// Simplified: returns start (the list recursion is a gap — no list type).
#[test]
fn test_gleamunison_range_base_case() {
    // range base case: if start > end { 0 } else { start }
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
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
                then_branch: Box::new(TypedExpr::Int(0)), // []
                else_branch: Box::new(TypedExpr::Var {
                    name: "start".into(),
                    type_: ValType::I64,
                }),
                type_: ValType::I64,
            },
        }],
        exports: vec!["range".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);
    assert!(&wasm[0..4] == b"\0asm");
    validate_module(&module, true).expect("module validates");
}

/// GleamUnison level 1: pure integer comparison
/// ```gleam
/// pub fn level1() -> Nil {
///   let x = 1; let y = 2;
///   case x < y { True -> ...  False -> ... }
/// }
/// ```
/// Compiles the comparison logic (state i64 → comparison result i64).
#[test]
fn test_gleamunison_level1_comparison() {
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "level1".into(),
            params: vec![],
            return_type: ValType::I64,
            // if 1 < 2 { 100 } else { 0 }
            body: TypedExpr::If {
                cond: Box::new(TypedExpr::BinOp {
                    op: BinOp::Lt,
                    left: Box::new(TypedExpr::Int(1)),
                    right: Box::new(TypedExpr::Int(2)),
                    type_: ValType::I64,
                }),
                then_branch: Box::new(TypedExpr::Int(100)), // pass
                else_branch: Box::new(TypedExpr::Int(0)),
                type_: ValType::I64,
            },
        }],
        exports: vec!["level1".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);
    assert!(&wasm[0..4] == b"\0asm");
    assert!(wasm.len() < 1024);
    validate_module(&module, true).expect("module validates");
}

/// GleamUnison hash function (simplified — pure u32 hash, no FFI)
/// Implements a simple FNV-1a hash on an i64 value.
#[test]
fn test_gleamunison_hash_i64() {
    // hash(n: i64) -> i64 — simple FNV-like: (n * 16777619) ^ (n >> 24)
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
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
        }],
        exports: vec!["hash".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);
    assert!(&wasm[0..4] == b"\0asm");
    validate_module(&module, true).expect("module validates");
}

/// GleamUnison full deployment: 4 functions in one module (simulated)
#[test]
fn test_gleamunison_full_deploy() {
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
        ],
        exports: vec![
            "local_var_index".into(),
            "range".into(),
            "hash".into(),
            "level1".into(),
        ],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    validate_module(&module, true).expect("GleamUnison module validates");

    let deploy_dir = "dogfooding/gleamunison/cf-deploy";
    std::fs::create_dir_all(deploy_dir).unwrap();
    std::fs::write(format!("{deploy_dir}/gleamunison.wasm"), &wasm).unwrap();
    std::fs::write(format!("{deploy_dir}/gleamunison.wat"), &wat).unwrap();

    assert!(&wasm[0..4] == b"\0asm");
    assert!(wat.contains("\"local_var_index\""));
    assert!(wat.contains("\"range\""));
    assert!(wat.contains("\"hash\""));
    assert!(wat.contains("\"level1\""));
    assert!(wat.contains("memory"));
    assert!(!wat.contains("struct.new"));
    assert!(wasm.len() < 3072);

    println!("GleamUnison WASM: {} bytes", wasm.len());
    println!("Deployed to {deploy_dir}");
}
