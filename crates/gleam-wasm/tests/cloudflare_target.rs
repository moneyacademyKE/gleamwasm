use gleam_wasm::codegen::GleamModule;
/// Tests for the Cloudflare-compatible linear memory target.
/// Verifies: no GC instructions, valid WASM magic, memory export.
use gleam_wasm::codegen::compile_to_linear;
use gleam_wasm::emit::emit_wasm;

#[test]
fn test_cf_target_produces_valid_wasm() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    // Valid WASM magic
    assert!(&wasm[0..4] == b"\0asm", "invalid magic");
    assert_eq!(wasm[4..8], [0x01, 0x00, 0x00, 0x00], "invalid version");

    // Must have memory section (id=5)
    assert!(
        wasm.windows(2).any(|w| w[0] == 0x05),
        "missing memory section"
    );

    // No GC instructions in WAT
    assert!(!wat.contains("struct.new"));
    assert!(!wat.contains("ref.test"));
    assert!(!wat.contains("ref.cast"));
    assert!(!wat.contains("br_on_cast"));
    assert!(!wat.contains("(sub"));

    println!("CF-compatible WASM: {} bytes", wasm.len());
}

#[test]
fn test_cf_target_has_bump_allocator() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (_module, wat) = compile_to_linear(&module_def);

    // Verify builtin runtime functions
    assert!(wat.contains("$alloc"));
    assert!(wat.contains("$make_tagged"));
    assert!(wat.contains("$get_tag"));
    assert!(wat.contains("$get_payload"));
    assert!(wat.contains("memory"));
    assert!(wat.contains("(memory"));
}

#[test]
fn test_cf_target_exports_memory() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (_module, wat) = compile_to_linear(&module_def);
    assert!(wat.contains("(export \"memory\""));
}

#[test]
fn test_cf_target_size_bound() {
    let module_def = GleamModule {
        functions: vec![],
        exports: vec![],
        imports: vec![],
        adt_types: vec![],
    };

    let (module, _wat) = compile_to_linear(&module_def);
    let wasm = emit_wasm(&module);

    // Runtime + allocator should be under 1KB
    assert!(
        wasm.len() < 1024,
        "CF runtime too large: {} bytes (target < 1024)",
        wasm.len()
    );
}
