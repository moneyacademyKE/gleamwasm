use crate::ir::module::{FuncType, Import, ImportDesc};
use crate::ir::types::ValType;

pub fn build_web_imports(concat_idx: u32, equals_idx: u32, length_idx: u32) -> Vec<Import> {
    vec![
        Import {
            module: "wasm:js-string".into(),
            name: "concat".into(),
            desc: ImportDesc::Func {
                type_index: concat_idx,
            },
        },
        Import {
            module: "wasm:js-string".into(),
            name: "equals".into(),
            desc: ImportDesc::Func {
                type_index: equals_idx,
            },
        },
        Import {
            module: "wasm:js-string".into(),
            name: "length".into(),
            desc: ImportDesc::Func {
                type_index: length_idx,
            },
        },
    ]
}

pub fn register_web_builtins(module: &mut crate::ir::module::Module) -> (u32, u32, u32) {
    let extern_ref = ValType::Ref(crate::ir::types::RefType::RefNull(
        crate::ir::types::HEAP_TYPE_EXTERN,
    ));

    let concat_idx = module.add_func_type(FuncType {
        params: vec![extern_ref.clone(), extern_ref.clone()],
        results: vec![extern_ref.clone()],
    });

    let equals_idx = module.add_func_type(FuncType {
        params: vec![extern_ref.clone(), extern_ref.clone()],
        results: vec![ValType::I32],
    });

    let length_idx = module.add_func_type(FuncType {
        params: vec![extern_ref],
        results: vec![ValType::I32],
    });

    (concat_idx, equals_idx, length_idx)
}

pub fn register_wasi_string_type(module: &mut crate::ir::module::Module) -> u32 {
    use crate::ir::types::{ArrayType, FieldType, StructType, ValType};

    let array_ty = module.add_array_type(ArrayType {
        name: Some("$StringArray".into()),
        field: ValType::I32, // i8 stored as i32 for alignment
        mutable: false,
    });

    module.add_struct_type(StructType {
        name: Some("$GleamString".into()),
        supertype: None,
        fields: vec![
            FieldType {
                name: Some("$length".into()),
                type_: ValType::I32,
                mutable: false,
            },
            FieldType {
                name: Some("$bytes".into()),
                type_: ValType::Ref(crate::ir::types::RefType::Ref(
                    crate::ir::types::HeapType::TypeIndex(array_ty),
                )),
                mutable: false,
            },
        ],
    });

    array_ty
}
