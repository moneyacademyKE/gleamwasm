/// Dogfooding: Compile a Lustre-style counter's `update` function.
///
/// Lusre Gleam source:
/// ```
/// fn update(state: Int, action: Action) -> #(Int, Cmd) {
///   case action {
///     Incr -> #(state + 1, cmd.none())
///     Decr -> #(state - 1, cmd.none())
///   }
/// }
/// ```
use gleam_wasm::codegen::*;
use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::module::*;
use gleam_wasm::ir::types::*;

#[test]
fn test_lustre_update_zero_field_variants() {
    // Action = { Incr, Decr } — zero-field ADT, good test for empty structs
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_boxed_primitives();
    mapper.register_adt("Action", &[("Incr", vec![]), ("Decr", vec![])]);

    let wat = emit_wat(mapper.module());
    println!("{wat}");

    // Verify zero-field variants generate correctly
    assert!(wat.contains("(type $Action"));
    assert!(wat.contains("(type $Incr (sub $Action)"));
    assert!(wat.contains("(type $Decr (sub $Action)"));
}

#[test]
fn test_lustre_update_match_action() {
    // Translate: case action { Incr -> ...  Decr -> ... }
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_boxed_primitives();
    mapper.register_adt("Action", &[("Incr", vec![]), ("Decr", vec![])]);
    mapper.register_local("action".into(), 0);

    let scrutinee_type = ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT));

    let expr = TypedExpr::Match {
        scrutinee: Box::new(TypedExpr::Var {
            name: "action".into(),
            type_: scrutinee_type,
        }),
        cases: vec![
            MatchCase {
                variant_index: 1, // Incr — zero fields
                bindings: vec![],
                // state + 1 — simplified: just return 1 for now
                body: Box::new(TypedExpr::Int(1)),
            },
            MatchCase {
                variant_index: 2, // Decr — zero fields
                bindings: vec![],
                body: Box::new(TypedExpr::Int(-1)),
            },
        ],
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);
    let lines: Vec<String> = body.iter().map(|i| i.to_string()).collect();
    let joined = lines.join("\n");
    println!("{joined}");

    assert!(joined.contains("ref.test"));
    assert!(joined.contains("struct.get $1 $f0") || joined.contains("unreachable"));
}

#[test]
fn test_lustre_update_full_function() {
    // Full update(state, action) -> Int with match on Action
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_boxed_primitives();
    mapper.register_adt("Action", &[("Incr", vec![]), ("Decr", vec![])]);

    // Register locals for params
    // state: local 0 (i64), action: local 1 (ref null struct)
    mapper.register_local("state".into(), 0);
    mapper.register_local("action".into(), 1);

    let mut module_builder = Module::new();
    // Add state param
    let mut locals = vec![
        Local::param("state", ValType::I64),
        Local::param("action", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
    ];

    let func_type_idx = module_builder.add_func_type(FuncType {
        params: vec![
            ValType::I64,
            ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
        ],
        results: vec![ValType::I64],
    });

    // Build: case action { Incr -> state + 1, Decr -> state - 1 }
    let scrutinee_type = ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT));

    let expr = TypedExpr::Match {
        scrutinee: Box::new(TypedExpr::Var {
            name: "action".into(),
            type_: scrutinee_type.clone(),
        }),
        cases: vec![
            MatchCase {
                variant_index: 1, // Incr
                bindings: vec![],
                body: Box::new(TypedExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(TypedExpr::Var {
                        name: "state".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Int(1)),
                    type_: ValType::I64,
                }),
            },
            MatchCase {
                variant_index: 2, // Decr
                bindings: vec![],
                body: Box::new(TypedExpr::BinOp {
                    op: BinOp::Sub,
                    left: Box::new(TypedExpr::Var {
                        name: "state".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Int(1)),
                    type_: ValType::I64,
                }),
            },
        ],
        type_: ValType::I64,
    };

    let body = compile_expr(&mut mapper, &expr);

    // Add additional locals from compilation
    let additional = mapper.get_local_count().saturating_sub(2);
    for i in 0..additional {
        locals.push(Local::var(format!("$_t{}", i), ValType::I64));
    }

    module_builder.functions.push(Function {
        name: Some("update".into()),
        type_index: func_type_idx,
        locals,
        body,
    });

    module_builder.exports.push(Export {
        name: "update".into(),
        kind: ExportKind::Func(0),
    });

    let wat = emit_wat(&module_builder);
    println!("{wat}");

    assert!(wat.contains("ref.test"));
    assert!(wat.contains("i64.add"));
    assert!(wat.contains("i64.sub"));
    assert!(wat.contains("br 2"), "match exit branch");
}

#[test]
fn test_lustre_compile_via_compile_module() {
    // Use the compile_module entry point — this is the "real" API
    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "update".into(),
            params: vec![("state".into(), ValType::I64)],
            return_type: ValType::I64,
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

    let output = compile_module_with_opt(&module_def, gleam_wasm::Target::WasmWeb);
    println!("{}", output.wat);

    assert!(output.wat.contains("i64.add"));
    assert!(output.wat.contains("(export \"update\""));
    assert!(output.wat.contains("(type $Int")); // boxed primitives registered
    assert!(output.wat.contains("(type $Float")); // boxed primitives registered
}
