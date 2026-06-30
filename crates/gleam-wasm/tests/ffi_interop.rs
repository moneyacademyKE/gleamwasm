use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::Instr;
use gleam_wasm::ir::module::*;

#[test]
fn test_register_web_builtins() {
    use gleam_wasm::ffi::register_web_builtins;

    let mut module = Module::new();
    let (concat_idx, equals_idx, length_idx) = register_web_builtins(&mut module);

    assert!(concat_idx < equals_idx);
    assert!(equals_idx < length_idx);

    let wat = emit_wat(&module);
    println!("{wat}");

    assert!(wat.contains("(type (func"));
    assert_eq!(module.types.len(), 3);
}

#[test]
fn test_web_imports_wat_emission() {
    let mut module = Module::new();
    let (concat_idx, equals_idx, length_idx) = gleam_wasm::ffi::register_web_builtins(&mut module);

    module.imports = gleam_wasm::ffi::build_web_imports(concat_idx, equals_idx, length_idx);

    let wat = emit_wat(&module);
    println!("{wat}");

    assert!(wat.contains("(import \"wasm:js-string\" \"concat\""));
    assert!(wat.contains("(import \"wasm:js-string\" \"equals\""));
    assert!(wat.contains("(import \"wasm:js-string\" \"length\""));
}

#[test]
fn test_wasi_string_type() {
    use gleam_wasm::ffi::register_wasi_string_type;
    use gleam_wasm::ir::TypeDefKind;

    let mut module = Module::new();
    let _string_array_idx = register_wasi_string_type(&mut module);

    assert_eq!(module.types.len(), 2);

    match &module.types[0].kind {
        TypeDefKind::Array(at) => {
            assert_eq!(at.name.as_deref(), Some("$StringArray"));
        }
        _ => panic!("expected Array"),
    }

    match &module.types[1].kind {
        TypeDefKind::Struct(st) => {
            assert_eq!(st.name.as_deref(), Some("$GleamString"));
            assert_eq!(st.fields.len(), 2);
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn test_externref_instructions() {
    assert_eq!(Instr::ExternConvertAny.to_string(), "extern.convert_any");
    assert_eq!(Instr::AnyConvertExtern.to_string(), "any.convert_extern");
}
