use crate::Target;
use crate::ir::module::*;
use crate::ir::types::*;

pub struct TypeMapper {
    target: Target,
    module: Module,
    type_map: std::collections::HashMap<String, u32>,
    locals: std::collections::HashMap<String, u32>,
    functions: std::collections::HashMap<String, u32>,
    /// Maps type index → field types for ADT variant lookups
    variant_fields: std::collections::HashMap<u32, Vec<ValType>>,
}

impl TypeMapper {
    pub fn new(target: Target) -> Self {
        TypeMapper {
            target,
            module: Module::new(),
            type_map: std::collections::HashMap::new(),
            locals: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
            variant_fields: std::collections::HashMap::new(),
        }
    }

    pub fn into_module(self) -> Module {
        self.module
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn module_mut(&mut self) -> &mut Module {
        &mut self.module
    }

    pub fn target(&self) -> Target {
        self.target
    }

    /// Register a Gleam custom ADT. Returns the base type index.
    pub fn register_adt(&mut self, name: &str, variants: &[(&str, Vec<(&str, ValType)>)]) -> u32 {
        let base_idx = self.module.add_struct_type(StructType {
            name: Some(format!("${name}")),
            supertype: None,
            fields: vec![],
        });
        self.type_map.insert(name.to_string(), base_idx);

        for (variant_name, fields) in variants {
            let field_types: Vec<ValType> = fields.iter().map(|(_, ft)| ft.clone()).collect();
            let struct_type_idx = self.module.add_struct_type(StructType {
                name: Some(format!("${variant_name}")),
                supertype: Some(base_idx),
                fields: fields
                    .iter()
                    .map(|(fname, ftype)| FieldType {
                        name: Some(format!("${fname}")),
                        type_: ftype.clone(),
                        mutable: false,
                    })
                    .collect(),
            });
            self.variant_fields.insert(struct_type_idx, field_types);
        }

        base_idx
    }

    /// Register a tuple type. Returns the struct type index.
    pub fn register_tuple(&mut self, arity: usize, fields: Vec<ValType>) -> u32 {
        let name = format!("Tuple{arity}");
        self.register_tuple_inner(name, arity, fields)
    }

    /// Get an existing tuple type or register a new one. Returns the struct type index.
    pub fn get_or_register_tuple(&mut self, arity: usize, fields: Vec<ValType>) -> u32 {
        let name = format!("Tuple{arity}");
        if let Some(idx) = self.type_map.get(&name) {
            return *idx;
        }
        self.register_tuple_inner(name, arity, fields)
    }

    fn register_tuple_inner(&mut self, name: String, _arity: usize, fields: Vec<ValType>) -> u32 {
        let field_types: Vec<FieldType> = fields
            .into_iter()
            .enumerate()
            .map(|(i, ftype)| FieldType {
                name: Some(format!("$f{i}")),
                type_: ftype,
                mutable: false,
            })
            .collect();

        let idx = self.module.add_struct_type(StructType {
            name: Some(format!("${name}")),
            supertype: None,
            fields: field_types,
        });
        self.type_map.insert(name, idx);
        idx
    }

    /// Register a closure type. Returns the struct type index and the func type index.
    pub fn register_closure(
        &mut self,
        param_types: Vec<ValType>,
        result_type: ValType,
    ) -> (u32, u32) {
        let _sig_name = format!("closure_sig_{}", self.type_map.len());
        let func_type_index = self.module.add_func_type(FuncType {
            params: param_types,
            results: vec![result_type],
        });

        let closure_idx = self.module.add_struct_type(StructType {
            name: Some("$Closure".to_string()),
            supertype: None,
            fields: vec![
                FieldType {
                    name: Some("$code".into()),
                    type_: ValType::Ref(RefType::Ref(HeapType::TypeIndex(func_type_index))),
                    mutable: false,
                },
                FieldType {
                    name: Some("$env".into()),
                    type_: ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)),
                    mutable: false,
                },
            ],
        });

        self.type_map.insert("Closure".into(), closure_idx);
        (closure_idx, func_type_index)
    }

    /// Get the type index registered for a Gleam type name.
    pub fn get_type_index(&self, name: &str) -> Option<u32> {
        self.type_map.get(name).copied()
    }

    pub fn register_local(&mut self, name: String, index: u32) {
        self.locals.insert(name, index);
    }

    pub fn get_local_index(&self, name: &str) -> Option<u32> {
        self.locals.get(name).copied()
    }

    pub fn get_local_count(&self) -> u32 {
        self.locals.len() as u32
    }

    pub fn register_function(&mut self, name: String, index: u32) {
        self.functions.insert(name, index);
    }

    pub fn get_function_index(&self, name: &str) -> Option<u32> {
        self.functions.get(name).copied()
    }

    pub fn get_variant_field_type(
        &self,
        variant_type_index: u32,
        field_index: u32,
    ) -> Option<&ValType> {
        self.variant_fields
            .get(&variant_type_index)
            .and_then(|fields| fields.get(field_index as usize))
    }

    /// Get the boxed Wasm GC type for a Gleam primitive.
    pub fn boxed_type_for_primitive(primitive: &str) -> ValType {
        match primitive {
            "Int" => ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
            "Float" => ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT)),
            "Bool" => ValType::Ref(RefType::RefNull(HEAP_TYPE_I31)),
            "Nil" => ValType::Ref(RefType::RefNull(HEAP_TYPE_NONE)),
            "String" => ValType::Ref(RefType::RefNull(HEAP_TYPE_EXTERN)),
            _ => ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)),
        }
    }

    /// Get the unboxed Wasm GC type for a Gleam primitive (local values).
    pub fn unboxed_type_for_primitive(primitive: &str) -> ValType {
        match primitive {
            "Int" => ValType::I64,
            "Float" => ValType::F64,
            "Bool" => ValType::I32,
            _ => ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)),
        }
    }

    /// Register boxed types for Int and Float (struct wrappers over i64/f64).
    pub fn register_boxed_primitives(&mut self) -> (u32, u32) {
        let int_boxed = self.module.add_struct_type(StructType {
            name: Some("$Int".into()),
            supertype: None,
            fields: vec![FieldType {
                name: Some("$value".into()),
                type_: ValType::I64,
                mutable: false,
            }],
        });
        let float_boxed = self.module.add_struct_type(StructType {
            name: Some("$Float".into()),
            supertype: None,
            fields: vec![FieldType {
                name: Some("$value".into()),
                type_: ValType::F64,
                mutable: false,
            }],
        });
        self.type_map.insert("Int".into(), int_boxed);
        self.type_map.insert("Float".into(), float_boxed);
        (int_boxed, float_boxed)
    }

    pub fn clear_locals(&mut self) {
        self.locals.clear();
    }

    pub fn clear_functions(&mut self) {
        self.functions.clear();
    }
}
