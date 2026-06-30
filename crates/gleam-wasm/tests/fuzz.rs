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
}
