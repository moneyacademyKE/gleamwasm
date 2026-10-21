use proptest::prelude::*;

use gleam_wasm::Target;
use gleam_wasm::codegen::types::TypeMapper;
use gleam_wasm::ir::types::*;

proptest! {
    #[test]
    fn fuzz_type_mapper_doesnt_panic(name in "[a-zA-Z][a-zA-Z0-9_]{0,20}") {
        let mut mapper = TypeMapper::new(Target::WasmWeb);
        let idx = mapper.register_adt(
            &name,
            &[("A", vec![("x", ValType::I64)]), ("B", vec![])],
        );
        assert_eq!(idx, 0);
        let module = mapper.into_module();
        assert_eq!(module.types.len(), 3);
    }

    #[test]
    fn fuzz_register_tuple(_name in "[A-Za-z][a-zA-Z0-9_]{0,10}") {
        let mut mapper = TypeMapper::new(Target::WasmWeb);
        let idx = mapper.register_tuple(2, vec![ValType::I64, ValType::F64]);
        assert_eq!(idx, 0);
        assert_eq!(mapper.module().types.len(), 1);
    }

    #[test]
    fn fuzz_closure_registration(param_count in 0usize..5) {
        let mut mapper = TypeMapper::new(Target::WasmWeb);
        let params: Vec<ValType> = (0..param_count).map(|_| ValType::I64).collect();
        let (closure_idx, func_idx) = mapper.register_closure(params, ValType::I64);
        assert!(closure_idx > func_idx);
        assert!(mapper.module().types.len() >= 2);
    }

    #[test]
    fn fuzz_table_elem_data_manual_encoding(
        // (table_min, elem_offset, n_items) with offset + n_items <= table_min,
        // so the active element segment always fits the table by construction.
        shape in (1u32..8).prop_flat_map(|min| {
            (0..=min).prop_flat_map(move |off| (0..=(min - off)).prop_map(move |n| (min, off, n)))
        }),
        data_offset in 0u32..1024,
        data_bytes in proptest::collection::vec(any::<u8>(), 0..128),
    ) {
        use gleam_wasm::emit::encode_manual;
        use gleam_wasm::ir::module::{
            DataSegment, ElementSegment, FuncType, Function, Memory, Module, Table, TableKind,
        };
        use gleam_wasm::ir::types::Instr;

        let (table_min, elem_off, n_items) = shape;
        let mut module = Module::new();
        let ty = module.add_func_type(FuncType {
            params: vec![],
            results: vec![],
        });
        module.functions.push(Function {
            name: None,
            type_index: ty,
            locals: vec![],
            body: vec![Instr::Unreachable],
        });
        module.memories.push(Memory { min: 1, max: None });
        module.tables.push(Table {
            kind: TableKind::FuncRef,
            min: table_min,
            max: Some(table_min + 4),
        });
        module.elements.push(ElementSegment {
            offset: elem_off,
            items: vec![0; n_items as usize],
        });
        module.data.push(DataSegment {
            offset: data_offset,
            bytes: data_bytes,
        });

        // GAP #6 same-commit fuzz mandate: the hand-rolled encoder's
        // table/elem/data bytes must satisfy the spec validator.
        let bytes = encode_manual(&module);
        wasmparser::Validator::new()
            .validate_all(&bytes)
            .unwrap_or_else(|e| panic!("wasmparser rejected manual encoding: {e}"));
    }
}
