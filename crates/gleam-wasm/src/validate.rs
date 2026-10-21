use crate::ir::module::{ExportKind, Module, TypeDefKind};
use crate::ir::types::Instr;

/// Errors found during pre-emission module validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// A type index references a type that doesn't exist.
    InvalidTypeIndex(u32),
    /// A function references a type with index that doesn't exist.
    InvalidFunctionType {
        function: String,
        type_index: u32,
        max: u32,
    },
    /// A local.get/set/tee references an out-of-bounds local index.
    InvalidLocal {
        function: String,
        index: u32,
        max: u32,
    },
    /// A call/return_call references a non-existent function index.
    InvalidCall {
        function: String,
        callee: u32,
        max: u32,
    },
    /// A br/br_if/br_table targets a label depth beyond available blocks.
    InvalidBranch {
        function: String,
        depth: u32,
        max_depth: u32,
    },
    /// An export references a non-existent function index.
    InvalidExportFunction { name: String, index: u32, max: u32 },
    /// GC instructions found in a module that targets linear memory.
    GcInstructionInLinearModule { function: String, instr: String },
    /// A compare instruction receives an unreasonable branch depth.
    UnreasonableBranchDepth { function: String, depth: u32 },
    /// A call_indirect references a non-existent or non-func type.
    InvalidCallIndirectType { function: String, type_index: u32 },
    /// A call_indirect references a table that doesn't exist.
    InvalidTableIndex {
        function: String,
        table_index: u32,
        max: u32,
    },
    /// An element segment item references a non-existent function.
    InvalidElemFuncIndex { index: u32, max: u32 },
    /// A table's max is smaller than its min.
    InvalidTableLimits { min: u32, max: u32 },
    /// A data segment doesn't fit in the declared memory.
    DataSegmentOutOfMemory {
        offset: u32,
        len: u32,
        memory_bytes: u32,
    },
}

/// Validate a module before emission. Returns Ok(()) or the first error found.
pub fn validate_module(module: &Module, is_linear: bool) -> Result<(), ValidationError> {
    let _func_count = module.functions.len() as u32;
    let total_func_space = (module.imports.len() + module.functions.len()) as u32;

    // Validate type references in functions
    for func in &module.functions {
        if func.type_index >= module.types.len() as u32 {
            return Err(ValidationError::InvalidFunctionType {
                function: func.name.clone().unwrap_or_default(),
                type_index: func.type_index,
                max: module.types.len() as u32,
            });
        }
    }

    // Validate instructions in each function body
    for func in &module.functions {
        let func_name = func.name.clone().unwrap_or_default();

        // Count param locals for correct indexing
        let param_count = func.locals.iter().filter(|l| l.kind.is_param()).count() as u32;
        let max_body_local =
            (param_count + func.locals.iter().filter(|l| l.kind.is_var()).count() as u32).max(1);

        validate_instr_slice(
            &func.body,
            module,
            total_func_space,
            &func_name,
            max_body_local,
            is_linear,
            0,
        )?;
    }

    // Validate exports reference existing functions (global index space)
    for exp in &module.exports {
        let ExportKind::Func(idx) = exp.kind;
        if idx >= total_func_space {
            return Err(ValidationError::InvalidExportFunction {
                name: exp.name.clone(),
                index: idx,
                max: total_func_space,
            });
        }
    }

    // Validate table limits
    for t in &module.tables {
        if let Some(max) = t.max {
            if max < t.min {
                return Err(ValidationError::InvalidTableLimits { min: t.min, max });
            }
        }
    }

    // Validate element segments reference existing functions
    for seg in &module.elements {
        for &idx in &seg.items {
            if idx >= total_func_space {
                return Err(ValidationError::InvalidElemFuncIndex {
                    index: idx,
                    max: total_func_space,
                });
            }
        }
    }

    // Validate data segments fit in the declared memory
    if let Some(mem) = module.memories.first() {
        let memory_bytes = mem.min.saturating_mul(65_536);
        for seg in &module.data {
            if seg.offset.saturating_add(seg.bytes.len() as u32) > memory_bytes {
                return Err(ValidationError::DataSegmentOutOfMemory {
                    offset: seg.offset,
                    len: seg.bytes.len() as u32,
                    memory_bytes,
                });
            }
        }
    }

    Ok(())
}

fn validate_instr_slice(
    body: &[Instr],
    module: &Module,
    func_count: u32,
    func_name: &str,
    max_local: u32,
    is_linear: bool,
    depth: u32,
) -> Result<(), ValidationError> {
    for instr in body {
        match instr {
            Instr::LocalGet(i) | Instr::LocalSet(i) | Instr::LocalTee(i) => {
                if *i >= max_local {
                    return Err(ValidationError::InvalidLocal {
                        function: func_name.to_string(),
                        index: *i,
                        max: max_local,
                    });
                }
            }
            Instr::Call(i) | Instr::ReturnCall(i) | Instr::RefFunc(i) => {
                if *i >= func_count {
                    return Err(ValidationError::InvalidCall {
                        function: func_name.to_string(),
                        callee: *i,
                        max: func_count,
                    });
                }
            }
            Instr::CallRef(i) | Instr::ReturnCallRef(i) => {
                if *i >= module.types.len() as u32 {
                    return Err(ValidationError::InvalidTypeIndex(*i));
                }
            }
            Instr::CallIndirect {
                type_index,
                table_index,
            } => {
                let type_ok = module
                    .types
                    .get(*type_index as usize)
                    .is_some_and(|td| matches!(td.kind, TypeDefKind::Func(..)));
                if !type_ok {
                    return Err(ValidationError::InvalidCallIndirectType {
                        function: func_name.to_string(),
                        type_index: *type_index,
                    });
                }
                if *table_index >= module.tables.len() as u32 {
                    return Err(ValidationError::InvalidTableIndex {
                        function: func_name.to_string(),
                        table_index: *table_index,
                        max: module.tables.len() as u32,
                    });
                }
            }
            Instr::StructNew(i)
            | Instr::StructGet { type_index: i, .. }
            | Instr::StructSet { type_index: i, .. }
            | Instr::ArrayNew(i)
            | Instr::ArrayGet(i)
            | Instr::ArraySet(i)
            | Instr::Throw(i) => {
                if *i >= module.types.len() as u32 {
                    return Err(ValidationError::InvalidTypeIndex(*i));
                }
                if is_linear {
                    let kind = &module.types[*i as usize].kind;
                    let name: String = match kind {
                        TypeDefKind::Struct(s) => s.name.clone().unwrap_or_default(),
                        TypeDefKind::Array(a) => a.name.clone().unwrap_or_default(),
                        TypeDefKind::Func(ft) => {
                            format!("func({}->{})", ft.params.len(), ft.results.len())
                        }
                    };
                    return Err(ValidationError::GcInstructionInLinearModule {
                        function: func_name.to_string(),
                        instr: name,
                    });
                }
            }
            Instr::RefTest(_)
            | Instr::RefCast(_)
            | Instr::BrOnCast { .. }
            | Instr::BrOnCastFail { .. } => {
                if is_linear {
                    return Err(ValidationError::GcInstructionInLinearModule {
                        function: func_name.to_string(),
                        instr: format!("{instr:?}"),
                    });
                }
            }
            Instr::RefNull(_)
            | Instr::RefI31
            | Instr::ExternConvertAny
            | Instr::AnyConvertExtern => {
                if is_linear {
                    return Err(ValidationError::GcInstructionInLinearModule {
                        function: func_name.to_string(),
                        instr: format!("{instr:?}"),
                    });
                }
            }
            Instr::Br(i) | Instr::BrIf(i) => {
                if *i > depth {
                    return Err(ValidationError::InvalidBranch {
                        function: func_name.to_string(),
                        depth: *i,
                        max_depth: depth,
                    });
                }
            }
            Instr::BrTable { branches, default } => {
                let max_label = branches
                    .iter()
                    .chain(std::iter::once(default))
                    .max()
                    .copied()
                    .unwrap_or(0);
                if max_label > depth {
                    return Err(ValidationError::InvalidBranch {
                        function: func_name.to_string(),
                        depth: max_label,
                        max_depth: depth,
                    });
                }
            }
            Instr::Block(b) | Instr::Loop(b) => {
                validate_instr_slice(
                    &b.instrs,
                    module,
                    func_count,
                    func_name,
                    max_local,
                    is_linear,
                    depth + 1,
                )?;
            }
            Instr::If {
                then_branch,
                else_branch,
            } => {
                validate_instr_slice(
                    &then_branch.instrs,
                    module,
                    func_count,
                    func_name,
                    max_local,
                    is_linear,
                    depth + 1,
                )?;
                if let Some(else_blk) = else_branch {
                    validate_instr_slice(
                        &else_blk.instrs,
                        module,
                        func_count,
                        func_name,
                        max_local,
                        is_linear,
                        depth + 1,
                    )?;
                }
            }
            _ => {} // other instructions don't need validation
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Function;
    use crate::ir::module::FuncType;

    #[test]
    fn test_validate_empty_module() {
        let module = Module::new();
        assert!(validate_module(&module, false).is_ok());
        assert!(validate_module(&module, true).is_ok());
    }

    #[test]
    fn test_validate_invalid_local() {
        let mut module = Module::new();
        let ty = module.add_func_type(FuncType {
            params: vec![],
            results: vec![],
        });
        module.functions.push(Function {
            name: Some("test".into()),
            type_index: ty,
            locals: vec![],
            body: vec![Instr::LocalGet(5)], // local 5 doesn't exist
        });

        let err = validate_module(&module, false).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidLocal { .. }));
    }

    #[test]
    fn test_validate_invalid_call() {
        let mut module = Module::new();
        let ty = module.add_func_type(FuncType {
            params: vec![],
            results: vec![],
        });
        module.functions.push(Function {
            name: Some("test".into()),
            type_index: ty,
            locals: vec![],
            body: vec![Instr::Call(99)], // function 99 doesn't exist
        });

        let err = validate_module(&module, false).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidCall { .. }));
    }

    #[test]
    fn test_validate_gc_in_linear_module() {
        let mut module = Module::new();
        let ty = module.add_func_type(FuncType {
            params: vec![],
            results: vec![],
        });
        module.functions.push(Function {
            name: Some("test".into()),
            type_index: ty,
            locals: vec![],
            body: vec![Instr::StructNew(0)],
        });

        let err = validate_module(&module, true).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::GcInstructionInLinearModule { .. }
        ));
    }

    #[test]
    fn test_validate_invalid_branch_depth() {
        let mut module = Module::new();
        let ty = module.add_func_type(FuncType {
            params: vec![],
            results: vec![],
        });
        module.functions.push(Function {
            name: Some("test".into()),
            type_index: ty,
            locals: vec![],
            body: vec![Instr::Br(5)], // branch depth 5 exceeds 0
        });

        let err = validate_module(&module, false).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidBranch { .. }));
    }

    use crate::ir::module::{DataSegment, ElementSegment, Memory, Table, TableKind};
    use crate::ir::types::{Instr, Local, ValType};

    fn table_fixture() -> Module {
        let mut m = Module::new();
        let ty = m.add_func_type(FuncType {
            params: vec![ValType::I64],
            results: vec![ValType::I64],
        });
        m.functions.push(Function {
            name: Some("$double".into()),
            type_index: ty,
            locals: vec![Local::param("$x", ValType::I64)],
            body: vec![Instr::LocalGet(0), Instr::I64Const(2), Instr::I64Mul],
        });
        m.tables.push(Table {
            kind: TableKind::FuncRef,
            min: 1,
            max: None,
        });
        m.elements.push(ElementSegment {
            offset: 0,
            items: vec![0],
        });
        m
    }

    #[test]
    fn test_validate_table_elem_ok() {
        assert!(validate_module(&table_fixture(), true).is_ok());
    }

    #[test]
    fn test_validate_rejects_bad_table_limits() {
        let mut m = table_fixture();
        m.tables[0].min = 5;
        m.tables[0].max = Some(1);
        assert!(matches!(
            validate_module(&m, true),
            Err(ValidationError::InvalidTableLimits { .. })
        ));
    }

    #[test]
    fn test_validate_rejects_elem_out_of_range() {
        let mut m = table_fixture();
        m.elements[0].items = vec![7];
        assert!(matches!(
            validate_module(&m, true),
            Err(ValidationError::InvalidElemFuncIndex { .. })
        ));
    }

    #[test]
    fn test_validate_rejects_data_out_of_memory() {
        let mut m = table_fixture();
        m.memories.push(Memory { min: 1, max: None });
        m.data.push(DataSegment {
            offset: 65_536,
            bytes: vec![1, 2, 3],
        });
        assert!(matches!(
            validate_module(&m, true),
            Err(ValidationError::DataSegmentOutOfMemory { .. })
        ));
    }

    #[test]
    fn test_validate_rejects_call_indirect_bad_type() {
        let mut m = table_fixture();
        m.functions[0]
            .body
            .push(Instr::CallIndirect {
                type_index: 99,
                table_index: 0,
            });
        assert!(matches!(
            validate_module(&m, true),
            Err(ValidationError::InvalidCallIndirectType { .. })
        ));
    }
}
