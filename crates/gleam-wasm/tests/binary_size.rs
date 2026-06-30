use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::module::*;
use gleam_wasm::ir::types::*;

/// Build a minimal hello-world module and check its WAT size.
#[test]
fn test_hello_world_module_size() {
    let mut module = Module::new();

    let func_type_idx = module.add_func_type(FuncType {
        params: vec![],
        results: vec![],
    });

    module.functions.push(Function {
        name: Some("$hello".into()),
        type_index: func_type_idx,
        locals: vec![],
        body: vec![],
    });

    module.exports.push(Export {
        name: "hello".into(),
        kind: ExportKind::Func(0),
    });

    let wat = emit_wat(&module);
    let wat_bytes = wat.len();

    // Hello World module WAT should be well under 1KB
    assert!(
        wat_bytes < 500,
        "hello world WAT is {wat_bytes} bytes, expected < 500"
    );

    // Build the binary size test module — a list map/filter
    assert!(wat.contains("(module\n"));
    assert!(wat.contains("(func $hello"));
    assert!(wat.contains("(export \"hello\""));
}

#[test]
fn test_tuple_type_compilation() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);
    let _tuple_idx = mapper.register_tuple(
        3,
        vec![
            ValType::I64,
            ValType::F64,
            ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)),
        ],
    );

    let module = mapper.into_module();
    let wat = emit_wat(&module);

    assert!(wat.contains("(type $Tuple3"));
    assert!(wat.contains("(field $f0 i64)"));
    assert!(wat.contains("(field $f1 f64)"));
    assert!(wat.contains("(field $f2 (ref null any))"));
}

#[test]
fn test_result_type_pattern() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);
    mapper.register_adt(
        "Result",
        &[
            ("Ok", vec![("value", ValType::I64)]),
            ("Err", vec![("error", ValType::I64)]),
        ],
    );

    let module = mapper.into_module();
    let wat = emit_wat(&module);

    assert!(wat.contains("(type $Result"));
    assert!(wat.contains("(type $Ok (sub $Result)"));
    assert!(wat.contains("(type $Err (sub $Result)"));
}

#[test]
fn test_option_type_pattern() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);
    mapper.register_adt(
        "Option",
        &[
            (
                "Some",
                vec![("value", ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)))],
            ),
            ("None_", vec![]),
        ],
    );

    let module = mapper.into_module();
    let wat = emit_wat(&module);

    assert!(wat.contains("(type $Option"));
    assert!(wat.contains("(type $Some (sub $Option)"));
    assert!(wat.contains("(type $None_ (sub $Option)"));
    assert!(wat.contains("(field $value (ref null any))"));
}

#[test]
fn test_list_pattern_codegen() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);
    let _list_idx = mapper.register_adt(
        "List",
        &[
            (
                "Cons",
                vec![
                    ("head", ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY))),
                    ("tail", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
                ],
            ),
            ("Nil", vec![]),
        ],
    );

    let module = mapper.into_module();
    let wat = emit_wat(&module);

    assert!(wat.contains("(type $List"));
    assert!(wat.contains("(type $Cons (sub $List)"));
    assert!(wat.contains("(type $Nil (sub $List)"));
    assert!(wat.contains("(field $head (ref null any))"));
    assert!(wat.contains("(field $tail (ref null struct))"));
}

#[test]
fn test_full_prelude_size_estimate() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);

    // Register typical prelude types
    mapper.register_adt("Bool", &[("True", vec![]), ("False", vec![])]);
    mapper.register_adt(
        "Option",
        &[
            (
                "Some",
                vec![("value", ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)))],
            ),
            ("None_", vec![]),
        ],
    );
    mapper.register_adt(
        "Result",
        &[
            ("Ok", vec![("value", ValType::I64)]),
            ("Err", vec![("error", ValType::I64)]),
        ],
    );
    mapper.register_adt(
        "List",
        &[
            (
                "Cons",
                vec![
                    ("head", ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY))),
                    ("tail", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
                ],
            ),
            ("Nil", vec![]),
        ],
    );

    mapper.register_tuple(
        2,
        vec![
            ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)),
            ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)),
        ],
    );

    let wat = emit_wat(mapper.module());

    // Full prelude WAT should be well under 15KB
    let size = wat.len();
    assert!(size < 3000, "prelude WAT is {size} bytes, expected < 3000");
}
