use gleam_wasm::codegen::*;
use gleam_wasm::ir::types::*;
use std::fs;

#[test]
fn test_compile_lustre_counter_and_write_wasm() {
    // Build the Lustre counter's update function
    let mut mapper = TypeMapper::new(gleam_wasm::Target::WasmWeb);
    mapper.register_boxed_primitives();
    mapper.register_adt("Action", &[("Incr", vec![]), ("Decr", vec![])]);

    mapper.register_local("state".into(), 0);
    mapper.register_local("action".into(), 1);

    let scrutinee_type = ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT));

    let module_def = GleamModule {
        functions: vec![GleamFunctionDef {
            name: "update".into(),
            params: vec![
                ("state".into(), ValType::I64),
                (
                    "action".into(),
                    ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
                ),
            ],
            return_type: ValType::I64,
            body: TypedExpr::Match {
                scrutinee: Box::new(TypedExpr::Var {
                    name: "action".into(),
                    type_: scrutinee_type.clone(),
                }),
                cases: vec![
                    MatchCase {
                        variant_index: 1,
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
                        variant_index: 2,
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
            },
        }],
        exports: vec!["update".into()],
        imports: vec![],
        adt_types: vec![(
            "Action".into(),
            vec![("Incr".into(), vec![]), ("Decr".into(), vec![])],
        )],
    };

    let output = compile_module_with_opt(&module_def, gleam_wasm::Target::WasmWeb);

    // Write files
    let out_dir = "dogfooding/lustre-counter/output";
    fs::create_dir_all(out_dir).unwrap();

    let wat_path = format!("{out_dir}/counter.wat");
    fs::write(&wat_path, &output.wat).unwrap();

    if let Some(wasm_bytes) = &output.wasm {
        let wasm_path = format!("{out_dir}/counter.wasm");
        fs::write(&wasm_path, wasm_bytes).unwrap();
        eprintln!("WASM written: {wasm_path} ({} bytes)", wasm_bytes.len());

        // Verify it's valid WASM
        assert!(&wasm_bytes[0..4] == b"\0asm", "invalid WASM magic");
        assert_eq!(
            wasm_bytes[4..8],
            [0x01, 0x00, 0x00, 0x00],
            "invalid WASM version"
        );

        // Check binary size target (< 2KB for counter app)
        assert!(
            wasm_bytes.len() < 2048,
            "binary too large: {} bytes",
            wasm_bytes.len()
        );
    } else {
        eprintln!("WASM not produced (wast encode fallback)");
    }

    // Verify WAT contents
    assert!(output.wat.contains("(type $Int"));
    assert!(output.wat.contains("(type $Float"));
    assert!(output.wat.contains("i64.add"));
    assert!(output.wat.contains("i64.sub"));
    assert!(output.wat.contains("ref.test"));
    assert!(output.wat.contains("br 2"));
    assert!(output.wat.contains("(export \"update\""));
}
