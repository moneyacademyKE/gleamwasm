//! Self-contained GleamUnison WASM runtime.
//!
//! All 12 FFI stubs are implemented in pure WASM MVP instructions.
//! Zero JavaScript imports required — the resulting `.wasm` can run
//! on any MVP-compliant runtime (Cloudflare Workers, wasmtime, Node.js, Deno).

mod rt_core;
mod rt_services;

use crate::codegen::GleamModule;
use crate::codegen::linear::FunctionContext;
use crate::ir::module::{
    Export, ExportKind, FuncType, Function, Global, GlobalType, Memory, Module,
};
use crate::ir::types::{Instr, Local, ValType};
use std::collections::BTreeMap;

pub struct SelfContainedAdapter {
    pub module: Module,
    pub wat: String,
}

/// Build a fully self-contained GleamUnison WASM module.
/// All FFI calls are implemented in pure WASM — no JS imports.
pub fn compile_self_contained(module_def: &GleamModule) -> SelfContainedAdapter {
    let mut module = Module::new();

    // --- Linear memory (2 pages = 128KB) ---
    module.memories.push(Memory {
        min: 2,
        max: Some(256),
    });

    // --- Globals ---
    // global 0: heap pointer (mutable i32)
    module.globals.push(Global {
        type_: GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        init: vec![Instr::I32Const(256)], // start at 256 to leave room for header
    });

    // --- Self-contained runtime builtins (indices 0-16) ---
    rt_core::push_core_builtins(&mut module);
    rt_services::push_service_builtins(&mut module);

    let builtin_count = module.functions.len() as u32;
    let function_map: BTreeMap<String, u32> = module_def
        .functions
        .iter()
        .enumerate()
        .map(|(i, func)| (func.name.clone(), builtin_count + i as u32))
        .collect();

    // --- User functions ---
    for (i, func) in module_def.functions.iter().enumerate() {
        let func_index = builtin_count + i as u32;
        let func_type_idx = module.add_func_type(FuncType {
            params: vec![ValType::I32; func.params.len()],
            results: vec![ValType::I32],
        });

        let param_count = func.params.len() as u32;
        let mut ctx = FunctionContext {
            func_index_offset: builtin_count,
            local_offset: param_count,
            var_map: BTreeMap::new(),
            runtime_alloc: 0,
            runtime_free: 17,
            runtime_box_int: 1,
            runtime_unbox_int: 2,
            runtime_make_tagged: 1,
            runtime_get_tag: 2,
            runtime_get_payload: 3,
            runtime_nil: 17,
            runtime_cons: 18,
            runtime_list_head: 19,
            runtime_list_tail: 20,
            function_map: function_map.clone(),
        };
        for (i, (name, _)) in func.params.iter().enumerate() {
            ctx.var_map.insert(name.clone(), i as u32);
        }

        let compiled = crate::codegen::linear::compile_linear_expr(&func.body, &module, &ctx);

        let mut locals: Vec<Local> = func
            .params
            .iter()
            .map(|(name, _): &(String, ValType)| Local::param(name, ValType::I32))
            .collect();
        for i in 0..4 {
            locals.push(Local::var(format!("$tmp{i}"), ValType::I32));
        }
        for i in 0..2 {
            locals.push(Local::var(format!("$ftmp{i}"), ValType::F64));
        }

        module.functions.push(Function {
            name: Some(func.name.clone()),
            type_index: func_type_idx,
            locals,
            body: compiled,
        });

        module.exports.push(Export {
            name: func.name.clone(),
            kind: ExportKind::Func(func_index),
        });
    }

    let wat = crate::emit::wat::emit_wat(&module);
    SelfContainedAdapter { module, wat }
}
