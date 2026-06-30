use gleam_wasm::codegen::ast::*;
/// Dogfood: Compile a simplified Gleam CMS to Cloudflare WASM.
///
/// This test translates key CMS operations (add_page, find_page, publish_page)
/// into TypedExpr and compiles them for the Cloudflare target.
///
/// The real Gleam CMS source is at dogfooding/gleam_cms/src/cms.gleam
use gleam_wasm::codegen::*;
use gleam_wasm::emit::emit_wasm;
use gleam_wasm::ir::types::*;
use gleam_wasm::validate::validate_module;

#[test]
fn test_cms_init_returns_zero() {
    // init() -> Int (page count = 0)
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "init".into(),
            params: vec![],
            return_type: ValType::I64,
            body: TypedExpr::Int(0),
        }],
        exports: vec!["init".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(wat.contains("\"init\""));
    assert!(wat.contains("$alloc"));
    assert!(wat.contains("memory"));
    assert!(wat.contains("i32.const 0"));

    validate_module(&module, true).expect("module should validate");
}

#[test]
fn test_cms_add_page_simple() {
    // Simplified: add_page(count: Int) -> Result(Int, String)
    // Returns Ok(count + 1) — simulating successful page addition
    //
    // Real Gleam: add_page(state, title, slug, body) -> Result(CmsState, String)
    // Simplified: we just increment the count (string title/slug/body not yet representable)

    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "add_page".into(),
            params: vec![("count".into(), ValType::I64)],
            return_type: ValType::I64,
            // body = count + 1
            body: TypedExpr::BinOp {
                op: BinOp::Add,
                left: Box::new(TypedExpr::Var {
                    name: "count".into(),
                    type_: ValType::I64,
                }),
                right: Box::new(TypedExpr::Int(1)),
                type_: ValType::I64,
            },
        }],
        exports: vec!["add_page".into()],
        imports: vec![],
        adt_types: vec![
            // Result type: Ok(Int) | Error(String-placeholder)
            // We register this as an ADT so the type mapper knows about it
            (
                "Result".into(),
                vec![
                    ("Ok".into(), vec![("value".into(), ValType::I64)]),
                    ("Error".into(), vec![]), // string not yet representable
                ],
            ),
        ],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(wat.contains("\"add_page\""));
    assert!(wat.contains("i32.add"));
    assert!(!wat.contains("struct.new")); // no GC instructions

    validate_module(&module, true).expect("module should validate");

    println!(
        "CMS add_page WASM: {} bytes, WAT: {} bytes",
        wasm.len(),
        wat.len()
    );
}

#[test]
fn test_cms_find_page_option() {
    // find_page(count: Int, id: Int) -> Int
    // Simplified: if id < count then id (Some) else -1 (None sentinel)
    //
    // Real Gleam: find_page(state, slug) -> Option(Page)
    // Uses list.find with string comparison
    // Gap: strings, lists, record types

    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "find_page".into(),
            params: vec![
                ("count".into(), ValType::I64),
                ("id".into(), ValType::I64),
            ],
            return_type: ValType::I64,
            // if id < count { id } else { -1 }
            body: TypedExpr::If {
                cond: Box::new(TypedExpr::BinOp {
                    op: BinOp::Lt,
                    left: Box::new(TypedExpr::Var {
                        name: "id".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Var {
                        name: "count".into(),
                        type_: ValType::I64,
                    }),
                    type_: ValType::I64,
                }),
                then_branch: Box::new(TypedExpr::Var {
                    name: "id".into(),
                    type_: ValType::I64,
                }),
                else_branch: Box::new(TypedExpr::Int(-1)),
                type_: ValType::I64,
            },
        }],
        exports: vec!["find_page".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(wat.contains("\"find_page\""));
    assert!(wat.contains("i32.lt_s")); // comparison

    validate_module(&module, true).expect("module should validate");

    println!("CMS find_page WASM: {} bytes", wasm.len());
}

#[test]
fn test_cms_publish_page_match() {
    // publish_page(count: Int, action: Int) -> Int
    // action is a simplified ADT: 0 = Publish(id), 1 = NoOp
    // Returns updated count
    //
    // Real Gleam: publish_page(state, slug) -> Result(CmsState, String)
    // Pattern matches on find_page result, updates record field

    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "publish_page".into(),
            params: vec![
                ("count".into(), ValType::I64),
                ("action".into(), ValType::I64),
            ],
            return_type: ValType::I64,
            // match action { 0 => count, _ => count + 1 }
            body: TypedExpr::Match {
                scrutinee: Box::new(TypedExpr::Var {
                    name: "action".into(),
                    type_: ValType::I64,
                }),
                cases: vec![
                    MatchCase {
                        variant_index: 0, // Publish(id)
                        bindings: vec![],
                        body: Box::new(TypedExpr::Var {
                            name: "count".into(),
                            type_: ValType::I64,
                        }),
                    },
                    MatchCase {
                        variant_index: 1, // NoOp
                        bindings: vec![],
                        body: Box::new(TypedExpr::BinOp {
                            op: BinOp::Add,
                            left: Box::new(TypedExpr::Var {
                                name: "count".into(),
                                type_: ValType::I64,
                            }),
                            right: Box::new(TypedExpr::Int(1)),
                            type_: ValType::I64,
                        }),
                    },
                ],
                type_: ValType::I64,
            },
        }],
        exports: vec!["publish_page".into()],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    assert!(&wasm[0..4] == b"\0asm");
    assert!(wat.contains("\"publish_page\""));
    assert!(wat.contains("i32.eq")); // tag comparison for match

    validate_module(&module, true).expect("module should validate");

    println!("CMS publish_page WASM: {} bytes", wasm.len());
}

#[test]
fn test_cms_full_deploy() {
    // All CMS functions in a single module.
    // This is the closest approximation to a real Gleam CMS backend
    // that can be deployed to Cloudflare Workers.

    let module_def = GleamModule {
        functions: vec![
            GleamFunctionDef {
                name: "init".into(),
                params: vec![],
                return_type: ValType::I64,
                body: TypedExpr::Int(0),
            },
            GleamFunctionDef {
                name: "add_page".into(),
                params: vec![("count".into(), ValType::I64)],
                return_type: ValType::I64,
                body: TypedExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(TypedExpr::Var {
                        name: "count".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Int(1)),
                    type_: ValType::I64,
                },
            },
            GleamFunctionDef {
                name: "find_page".into(),
                params: vec![
                    ("count".into(), ValType::I64),
                    ("id".into(), ValType::I64),
                ],
                return_type: ValType::I64,
                body: TypedExpr::If {
                    cond: Box::new(TypedExpr::BinOp {
                        op: BinOp::Lt,
                        left: Box::new(TypedExpr::Var {
                            name: "id".into(),
                            type_: ValType::I64,
                        }),
                        right: Box::new(TypedExpr::Var {
                            name: "count".into(),
                            type_: ValType::I64,
                        }),
                        type_: ValType::I64,
                    }),
                    then_branch: Box::new(TypedExpr::Var {
                        name: "id".into(),
                        type_: ValType::I64,
                    }),
                    else_branch: Box::new(TypedExpr::Int(-1)),
                    type_: ValType::I64,
                },
            },
            GleamFunctionDef {
                name: "published_count".into(),
                params: vec![("count".into(), ValType::I64)],
                return_type: ValType::I64,
                body: TypedExpr::Var {
                    name: "count".into(),
                    type_: ValType::I64,
                },
            },
        ],
        exports: vec![
            "init".into(),
            "add_page".into(),
            "find_page".into(),
            "published_count".into(),
        ],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    // Validate before emission
    validate_module(&module, true).expect("CMS module should validate");

    // Write output for wrangler deployment
    let deploy_dir = "dogfooding/gleam_cms/cf-deploy";
    std::fs::create_dir_all(deploy_dir).unwrap();
    std::fs::write(format!("{deploy_dir}/cms.wasm"), &wasm).unwrap();
    std::fs::write(format!("{deploy_dir}/cms.wat"), &wat).unwrap();

    assert!(&wasm[0..4] == b"\0asm");
    assert_eq!(wasm[4..8], [0x01, 0x00, 0x00, 0x00]);

    // Verify all exports
    assert!(wat.contains("\"init\""));
    assert!(wat.contains("\"add_page\""));
    assert!(wat.contains("\"find_page\""));
    assert!(wat.contains("\"published_count\""));

    // No GC instructions
    assert!(!wat.contains("struct.new"));
    assert!(!wat.contains("ref.test"));
    assert!(!wat.contains("br_on_cast"));

    // Has runtime
    assert!(wat.contains("$alloc"));
    assert!(wat.contains("memory"));

    // Size check: multi-function CMS under 3KB
    assert!(
        wasm.len() < 3072,
        "CMS module too large: {} bytes (target < 3072)",
        wasm.len()
    );

    println!("CMS WASM: {} bytes", wasm.len());
    println!("CMS deployed to {deploy_dir}");
}
