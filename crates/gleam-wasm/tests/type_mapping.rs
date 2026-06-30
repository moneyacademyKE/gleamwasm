use gleam_wasm::Target;
use gleam_wasm::ir::module::TypeDefKind;
use gleam_wasm::ir::{HEAP_TYPE_ANY, RefType, ValType};

#[test]
fn test_register_shape_adt() {
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);
    let base_idx = mapper.register_adt(
        "Shape",
        &[
            ("Circle", vec![("radius", ValType::F64)]),
            (
                "Rect",
                vec![("width", ValType::F64), ("height", ValType::F64)],
            ),
        ],
    );

    let module = mapper.into_module();
    assert_eq!(module.types.len(), 3);
    assert_eq!(base_idx, 0);

    match &module.types[0].kind {
        TypeDefKind::Struct(st) => {
            assert_eq!(st.name.as_deref(), Some("$Shape"));
            assert!(st.supertype.is_none());
            assert!(st.fields.is_empty());
        }
        _ => panic!("expected Struct"),
    }

    match &module.types[1].kind {
        TypeDefKind::Struct(st) => {
            assert_eq!(st.name.as_deref(), Some("$Circle"));
            assert_eq!(st.supertype, Some(0));
            assert_eq!(st.fields.len(), 1);
            assert_eq!(st.fields[0].type_, ValType::F64);
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_primitive_boxed_types() {
    use gleam_wasm::codegen::types::TypeMapper;

    assert_eq!(
        TypeMapper::boxed_type_for_primitive("Int").to_string(),
        "(ref null struct)"
    );
    assert_eq!(
        TypeMapper::boxed_type_for_primitive("Bool").to_string(),
        "(ref null i31)"
    );
    assert_eq!(
        TypeMapper::boxed_type_for_primitive("Nil").to_string(),
        "(ref null none)"
    );
}

#[test]
fn test_register_closure() {
    use gleam_wasm::codegen::types::TypeMapper;

    let mut mapper = TypeMapper::new(Target::WasmWeb);
    let (closure_idx, func_type_idx) = mapper.register_closure(
        vec![ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)), ValType::I64],
        ValType::I64,
    );

    let module = mapper.into_module();
    assert_eq!(closure_idx, 1);
    assert_eq!(func_type_idx, 0);

    match &module.types[0].kind {
        TypeDefKind::Func(ft) => {
            assert_eq!(ft.params.len(), 2);
            assert_eq!(ft.results.len(), 1);
            assert_eq!(ft.results[0], ValType::I64);
        }
        _ => panic!("expected Func"),
    }
}
