use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::module::*;
use gleam_wasm::ir::{
    Export, ExportKind, FieldType, FuncType, Instr, Local, Module, StructType, ValType,
};

#[test]
fn test_emit_simple_module() {
    let mut module = Module::new();

    module.add_struct_type(StructType {
        name: Some("$Int".into()),
        supertype: None,
        fields: vec![FieldType {
            name: Some("$value".into()),
            type_: ValType::I64,
            mutable: false,
        }],
    });

    let func_type_idx = module.add_func_type(FuncType {
        params: vec![ValType::I64, ValType::I64],
        results: vec![ValType::I64],
    });

    module.functions.push(Function {
        name: Some("$add".into()),
        type_index: func_type_idx,
        locals: vec![
            Local::param("$a", ValType::I64),
            Local::param("$b", ValType::I64),
        ],
        body: vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::I64Add],
    });

    module.exports.push(Export {
        name: "add".into(),
        kind: ExportKind::Func(0),
    });

    let wat = emit_wat(&module);
    println!("{wat}");

    assert!(wat.contains("(module\n"));
    assert!(wat.contains("(type $Int"));
    assert!(wat.contains("(func $add"));
    assert!(wat.contains("i64.add"));
    assert!(wat.contains("(export \"add\" (func 0))"));
}

#[test]
fn test_emit_adt_with_subtyping() {
    use gleam_wasm::Target;
    use gleam_wasm::codegen::types::TypeMapper;

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

    let module = mapper.into_module();
    let wat = emit_wat(&module);
    println!("{wat}");

    assert!(wat.contains("(type $Shape"));
    assert!(wat.contains("(type $Circle (sub $Shape)"));
    assert!(wat.contains("(type $Rect (sub $Shape)"));
    assert!(wat.contains("(field $radius f64)"));
    assert!(wat.contains("(field $width f64)"));
    assert!(wat.contains("(field $height f64)"));
}
