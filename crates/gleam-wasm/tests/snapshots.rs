use gleam_wasm::Target;
use gleam_wasm::codegen::types::TypeMapper;
use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::types::*;

#[test]
fn test_snapshot_shape_adt() {
    let mut mapper = TypeMapper::new(Target::WasmWeb);
    mapper.register_adt(
        "Shape",
        &[
            ("Circle", vec![("radius", ValType::F64)]),
            (
                "Rect",
                vec![("width", ValType::F64), ("height", ValType::F64)],
            ),
        ],
    );

    let wat = emit_wat(mapper.module());
    insta::assert_snapshot!(wat);
}

#[test]
fn test_snapshot_full_prelude() {
    let mut mapper = TypeMapper::new(Target::WasmWeb);

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
    insta::assert_snapshot!(wat);
}
