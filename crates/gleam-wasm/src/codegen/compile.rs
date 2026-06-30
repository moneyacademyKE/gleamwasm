use crate::Target;
use crate::ir::module::{Export, ExportKind, FuncType, Function as IrFunction};
use crate::ir::types::{Local, ValType};
use crate::wasm_opt::WasmOpt;

use super::ast::TypedExpr;
use super::expr::compile_expr;
use super::types::TypeMapper;

// Type aliases for ADT definitions
pub type AdtVariant = (String, Vec<(String, ValType)>);
pub type AdtDef = (String, Vec<AdtVariant>);

#[derive(Clone)]
pub struct GleamFunctionDef {
    pub name: String,
    pub params: Vec<(String, ValType)>,
    pub return_type: ValType,
    pub body: TypedExpr,
}

#[derive(Clone)]
pub struct GleamModule {
    pub functions: Vec<GleamFunctionDef>,
    pub exports: Vec<String>,
    pub imports: Vec<crate::ir::module::Import>,
    pub adt_types: Vec<AdtDef>,
}

pub struct CompileOutput {
    pub wat: String,
    pub wasm: Option<Vec<u8>>,
}

pub fn compile_function(mapper: &mut TypeMapper, def: &GleamFunctionDef) -> IrFunction {
    let func_type_idx = mapper.module_mut().add_func_type(FuncType {
        params: def.params.iter().map(|(_, t)| t.clone()).collect(),
        results: vec![def.return_type.clone()],
    });

    let mut locals: Vec<Local> = def
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, t))| {
            mapper.register_local(name.clone(), i as u32);
            Local::param(name, t.clone())
        })
        .collect();

    mapper.register_function(def.name.clone(), mapper.module().functions.len() as u32);

    let func_local_count = def.params.len();
    let body_instrs = compile_expr(mapper, &def.body);

    let additional = mapper
        .get_local_count()
        .saturating_sub(func_local_count as u32);

    for i in 0..additional {
        locals.push(Local::var(format!("$_t{}", i), ValType::I64));
    }

    IrFunction {
        name: Some(def.name.clone()),
        type_index: func_type_idx,
        locals,
        body: body_instrs,
    }
}

/// A closure definition extracted from an expression tree, to be compiled
/// as a standalone function.
#[derive(Debug, Clone)]
struct ClosureDef {
    params: Vec<(String, ValType)>,
    #[allow(dead_code)]
    captured: Vec<String>,
    body: TypedExpr,
    func_index: u32,
    func_type_index: u32,
}

/// Walk an expression tree, register all closure types, assign inner function
/// indices, and collect closure definitions for later compilation.
fn preprocess_closures(
    expr: &mut TypedExpr,
    closures: &mut Vec<ClosureDef>,
    next_func_idx: &mut u32,
    mapper: &mut TypeMapper,
) {
    if let TypedExpr::Closure {
        params,
        captured,
        body,
        type_,
        inner_func_idx,
    } = expr
    {
        let param_types: Vec<ValType> = params.iter().map(|(_, t)| t.clone()).collect();
        let (_, func_type_idx) = mapper.register_closure(param_types, type_.clone());

        let func_idx = *next_func_idx;
        *next_func_idx += 1;
        *inner_func_idx = Some(func_idx);

        closures.push(ClosureDef {
            params: params.clone(),
            captured: captured.clone(),
            body: (**body).clone(),
            func_index: func_idx,
            func_type_index: func_type_idx,
        });
    }

    // Recurse into child expressions
    match expr {
        TypedExpr::Closure { body, .. } => {
            preprocess_closures(body, closures, next_func_idx, mapper);
        }
        TypedExpr::BinOp { left, right, .. } => {
            preprocess_closures(left, closures, next_func_idx, mapper);
            preprocess_closures(right, closures, next_func_idx, mapper);
        }
        TypedExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            preprocess_closures(cond, closures, next_func_idx, mapper);
            preprocess_closures(then_branch, closures, next_func_idx, mapper);
            preprocess_closures(else_branch, closures, next_func_idx, mapper);
        }
        TypedExpr::Let { value, body, .. } => {
            preprocess_closures(value, closures, next_func_idx, mapper);
            preprocess_closures(body, closures, next_func_idx, mapper);
        }
        TypedExpr::Call { args, .. }
        | TypedExpr::CallClosure { args, .. }
        | TypedExpr::TailCall { args, .. } => {
            for arg in args {
                preprocess_closures(arg, closures, next_func_idx, mapper);
            }
        }
        TypedExpr::StructNew { fields, .. } | TypedExpr::Tuple { elements: fields, .. } => {
            for field in fields {
                preprocess_closures(field, closures, next_func_idx, mapper);
            }
        }
        TypedExpr::StructGet { expr, .. } | TypedExpr::TupleGet { tuple: expr, .. } => {
            preprocess_closures(expr, closures, next_func_idx, mapper);
        }
        TypedExpr::Match {
            scrutinee, cases, ..
        } => {
            preprocess_closures(scrutinee, closures, next_func_idx, mapper);
            for case in cases {
                preprocess_closures(&mut case.body.clone(), closures, next_func_idx, mapper);
            }
        }
        TypedExpr::ListCons { head, tail, .. } => {
            preprocess_closures(head, closures, next_func_idx, mapper);
            preprocess_closures(tail, closures, next_func_idx, mapper);
        }
        _ => {}
    }
}

/// Compile a closure body as a standalone function.
fn compile_closure_function(
    mapper: &mut TypeMapper,
    closure: &ClosureDef,
) -> IrFunction {
    let func_type_idx = closure.func_type_index;

    let mut locals: Vec<Local> = closure
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, t))| {
            mapper.register_local(name.clone(), i as u32);
            Local::param(name, t.clone())
        })
        .collect();

    mapper.register_function(
        format!("$closure_{}", closure.func_index),
        closure.func_index,
    );

    let func_local_count = closure.params.len();
    let body_instrs = compile_expr(mapper, &closure.body);

    let additional = mapper
        .get_local_count()
        .saturating_sub(func_local_count as u32);

    for i in 0..additional {
        locals.push(Local::var(format!("$_c{}", i), ValType::I64));
    }

    IrFunction {
        name: Some(format!("$closure_{}", closure.func_index)),
        type_index: func_type_idx,
        locals,
        body: body_instrs,
    }
}

pub fn compile_module_with_opt(def: &GleamModule, target: Target) -> CompileOutput {
    // === Phase 1: Type registration and closure discovery ===
    let mut mapper = TypeMapper::new(target);
    mapper.register_boxed_primitives();

    for (adt_name, variants) in &def.adt_types {
        let variants_refs: Vec<(&str, Vec<(&str, ValType)>)> = variants
            .iter()
            .map(|(name, fields)| {
                let field_refs: Vec<(&str, ValType)> = fields
                    .iter()
                    .map(|(fname, ftype)| (fname.as_str(), ftype.clone()))
                    .collect();
                (name.as_str(), field_refs)
            })
            .collect();
        mapper.register_adt(adt_name, &variants_refs);
    }

    // Preprocess closures: register closure types, assign function indices,
    // and collect closure definitions for later compilation.
    let mut all_closures: Vec<ClosureDef> = Vec::new();
    let func_count = def.functions.len() as u32;
    let mut next_func_idx = func_count;

    let mut preprocessed_functions: Vec<GleamFunctionDef> = def.functions.clone();
    for func in &mut preprocessed_functions {
        preprocess_closures(&mut func.body, &mut all_closures, &mut next_func_idx, &mut mapper);
    }

    // === Phase 2: Compile closure functions ===
    mapper.clear_locals();
    mapper.clear_functions();

    let mut closure_ir_funcs: Vec<IrFunction> = Vec::new();
    for closure in &all_closures {
        let ir_func = compile_closure_function(&mut mapper, closure);
        closure_ir_funcs.push(ir_func);
    }

    // === Phase 3: Compile main module functions ===
    mapper.clear_locals();
    mapper.clear_functions();

    let mut main_ir_funcs: Vec<IrFunction> = Vec::new();
    for func in &preprocessed_functions {
        let ir_func = compile_function(&mut mapper, func);
        main_ir_funcs.push(ir_func);
    }

    // Push into module: main functions first (indices 0..func_count for exports),
    // then closures (indices func_count..).
    for f in main_ir_funcs {
        mapper.module_mut().functions.push(f);
    }
    for f in closure_ir_funcs {
        mapper.module_mut().functions.push(f);
    }

    mapper.module_mut().imports.extend(def.imports.clone());

    for (i, name) in def.exports.iter().enumerate() {
        mapper.module_mut().exports.push(Export {
            name: name.clone(),
            kind: ExportKind::Func(i as u32),
        });
    }

    let wat = crate::emit::emit_wat(mapper.module());
    let wasm_binary = crate::emit::emit_wasm(mapper.module());

    let wasm = if wasm_binary.len() > 8 && &wasm_binary[0..4] == b"\0asm" {
        WasmOpt::optimize_for_min_size(&wasm_binary)
            .or::<Vec<u8>>(Ok(wasm_binary))
            .ok()
    } else {
        Some(wasm_binary)
    };

    CompileOutput { wat, wasm }
}

pub fn compile_module(def: &GleamModule, target: Target, emit_wat: bool) -> String {
    let output = compile_module_with_opt(def, target);
    if emit_wat { output.wat } else { String::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::ast::{BinOp, TypedExpr};

    #[test]
    fn test_compile_simple_module() {
        let module_def = GleamModule {
            functions: vec![GleamFunctionDef {
                name: "add".into(),
                params: vec![("a".into(), ValType::I64), ("b".into(), ValType::I64)],
                return_type: ValType::I64,
                body: TypedExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(TypedExpr::Var {
                        name: "a".into(),
                        type_: ValType::I64,
                    }),
                    right: Box::new(TypedExpr::Var {
                        name: "b".into(),
                        type_: ValType::I64,
                    }),
                    type_: ValType::I64,
                },
            }],
            exports: vec!["add".into()],
            imports: vec![],
            adt_types: vec![],
        };

        let wat = compile_module(&module_def, Target::WasmWeb, true);
        println!("{wat}");

        assert!(wat.contains("(module\n"));
        assert!(wat.contains("(type $Int"));
        assert!(wat.contains("(func"));
        assert!(wat.contains("\"add\""));
        assert!(wat.contains("i64.add"));
        assert!(wat.contains("(export \"add\""));
    }

    #[test]
    fn test_compile_multi_function_module() {
        let module_def = GleamModule {
            functions: vec![
                GleamFunctionDef {
                    name: "add_one".into(),
                    params: vec![("x".into(), ValType::I64)],
                    return_type: ValType::I64,
                    body: TypedExpr::BinOp {
                        op: BinOp::Add,
                        left: Box::new(TypedExpr::Var {
                            name: "x".into(),
                            type_: ValType::I64,
                        }),
                        right: Box::new(TypedExpr::Int(1)),
                        type_: ValType::I64,
                    },
                },
                GleamFunctionDef {
                    name: "double".into(),
                    params: vec![("x".into(), ValType::I64)],
                    return_type: ValType::I64,
                    body: TypedExpr::BinOp {
                        op: BinOp::Mul,
                        left: Box::new(TypedExpr::Var {
                            name: "x".into(),
                            type_: ValType::I64,
                        }),
                        right: Box::new(TypedExpr::Int(2)),
                        type_: ValType::I64,
                    },
                },
            ],
            exports: vec!["add_one".into(), "double".into()],
            imports: vec![],
            adt_types: vec![],
        };

        let wat = compile_module(&module_def, Target::WasmWeb, true);
        println!("{wat}");

        assert!(wat.contains("(export \"add_one\""));
        assert!(wat.contains("(export \"double\""));
        assert!(wat.contains("\"add_one\""));
        assert!(wat.contains("\"double\""));
    }
}
