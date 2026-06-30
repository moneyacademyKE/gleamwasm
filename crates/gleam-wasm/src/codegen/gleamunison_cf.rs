/// GleamUnison → Cloudflare Workers adapter layer.
///
/// Maps the 75 GleamUnison `@external(erlang, ...)` FFI calls to Cloudflare
/// Worker equivalents by compiling a WASM module that imports JS stubs.
use crate::codegen::linear::FunctionContext;
use crate::codegen::GleamModule;
use crate::ir::module::{
    Export, ExportKind, FuncType, Function, Global, GlobalType, Import, ImportDesc, Memory, Module,
};
use crate::ir::types::{Instr, Local, ValType};
use std::collections::BTreeMap;

/// Full GleamUnison CF adapter. Produces a WASM module that imports
/// JavaScript stubs for all GleamUnison FFI calls.
pub struct GleamUnisonAdapter {
    pub module: Module,
    pub wat: String,
    pub import_count: u32,
}

/// Build a GleamUnison-compatible WASM module with JS import stubs.
pub fn compile_gleamunison(module_def: &GleamModule) -> GleamUnisonAdapter {
    let mut module = Module::new();

    // --- Linear memory ---
    module.memories.push(Memory {
        min: 1,
        max: Some(256),
    });
    module.globals.push(Global {
        type_: GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        init: vec![Instr::I32Const(8)],
    });

    // --- JS import stubs for GleamUnison FFI ---
    // Each import maps to a JavaScript function in the worker.

    // Import 0: gleamunison_hash_bytes(ptr: i32, len: i32) -> i32 (returns ptr to hash bytes)
    let hash_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "hash_bytes".into(),
        desc: ImportDesc::Func {
            type_index: hash_ty,
        },
    });

    // Import 1: gleamunison_hex_to_bytes(ptr: i32, len: i32) -> i32
    let hex_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "hex_to_bytes".into(),
        desc: ImportDesc::Func {
            type_index: hex_ty,
        },
    });

    // Import 2: gleamunison_hash_equal(a_ptr: i32, a_len: i32, b_ptr: i32, b_len: i32) -> i32
    let heq_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "hash_equal".into(),
        desc: ImportDesc::Func {
            type_index: heq_ty,
        },
    });

    // Import 3: gleamunison_hash_to_hex(ptr: i32, len: i32) -> i32 (returns ptr to hex string)
    let h2h_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "hash_to_hex".into(),
        desc: ImportDesc::Func {
            type_index: h2h_ty,
        },
    });

    // Import 4: gleamunison_state_get(key_ptr: i32, key_len: i32) -> i32
    let sget_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "state_get".into(),
        desc: ImportDesc::Func {
            type_index: sget_ty,
        },
    });

    // Import 5: gleamunison_state_set(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32
    let sset_ty = module.add_func_type(FuncType {
        params: vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "state_set".into(),
        desc: ImportDesc::Func {
            type_index: sset_ty,
        },
    });

    // Import 6: gleamunison_file_read(path_ptr: i32, path_len: i32) -> i32
    let fread_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "file_read".into(),
        desc: ImportDesc::Func {
            type_index: fread_ty,
        },
    });

    // Import 7: gleamunison_file_write(path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32) -> i32
    let fwrite_ty = module.add_func_type(FuncType {
        params: vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "file_write".into(),
        desc: ImportDesc::Func {
            type_index: fwrite_ty,
        },
    });

    // Import 8: gleamunison_log(ptr: i32, len: i32) -> i32
    let log_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "log".into(),
        desc: ImportDesc::Func {
            type_index: log_ty,
        },
    });

    // Import 9: gleamunison_now_ms() -> i64
    let now_ty = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I64],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "now_ms".into(),
        desc: ImportDesc::Func {
            type_index: now_ty,
        },
    });

    // Import 10: gleamunison_timestamp() -> i32
    let ts_ty = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "timestamp".into(),
        desc: ImportDesc::Func {
            type_index: ts_ty,
        },
    });

    // Import 11: gleamunison_eval(expr_ptr: i32, expr_len: i32) -> i32
    let eval_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.imports.push(Import {
        module: "gleamunison".into(),
        name: "eval".into(),
        desc: ImportDesc::Func {
            type_index: eval_ty,
        },
    });

    let import_count = module.imports.len() as u32;

    // --- Runtime builtins ---

    // runtime 0: alloc(size: i32) -> i32
    let alloc_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$alloc".into()),
        type_index: alloc_ty,
        locals: vec![
            Local::param("$size", ValType::I32),
            Local::var("$ptr", ValType::I32),
        ],
        body: vec![
            Instr::GlobalGet(0),
            Instr::LocalTee(1),
            Instr::LocalGet(0),
            Instr::I32Add,
            Instr::GlobalSet(0),
            Instr::LocalGet(1),
        ],
    });

    // runtime 1: make_tagged(tag: i32, payload: i32) -> i32
    let mt_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$make_tagged".into()),
        type_index: mt_ty,
        locals: vec![
            Local::param("$tag", ValType::I32),
            Local::param("$payload", ValType::I32),
            Local::var("$ret", ValType::I32),
        ],
        body: vec![
            Instr::I32Const(8),
            Instr::Call(import_count), // alloc
            Instr::LocalTee(2),
            Instr::LocalGet(0),
            Instr::I32Store {
                offset: 0,
                align: 2,
            },
            Instr::LocalGet(2),
            Instr::LocalGet(1),
            Instr::I32Store {
                offset: 4,
                align: 2,
            },
            Instr::LocalGet(2),
        ],
    });

    // runtime 2: get_tag(ptr: i32) -> i32
    let gt_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$get_tag".into()),
        type_index: gt_ty,
        locals: vec![Local::param("$ptr", ValType::I32)],
        body: vec![Instr::LocalGet(0), Instr::I32Load {
            offset: 0,
            align: 2,
        }],
    });

    // runtime 3: get_payload(ptr: i32) -> i32
    let gp_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$get_payload".into()),
        type_index: gp_ty,
        locals: vec![Local::param("$ptr", ValType::I32)],
        body: vec![Instr::LocalGet(0), Instr::I32Load {
            offset: 4,
            align: 2,
        }],
    });

    // runtime 4: gleamunison_hash_wrapper(ptr: i32, len: i32) -> i32
    let ghw_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$hash_bytes".into()),
        type_index: ghw_ty,
        locals: vec![
            Local::param("$ptr", ValType::I32),
            Local::param("$len", ValType::I32),
        ],
        body: vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::Call(0)], // import 0
    });

    // runtime 5: gleamunison_hex_to_bytes_wrapper(ptr: i32, len: i32) -> i32
    let ht_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$hex_to_bytes".into()),
        type_index: ht_ty,
        locals: vec![
            Local::param("$ptr", ValType::I32),
            Local::param("$len", ValType::I32),
        ],
        body: vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::Call(1)], // import 1
    });

    // --- User functions ---
    let builtin_count = module.functions.len() as u32;
    let start_func_idx = builtin_count;

    for func in &module_def.functions {
        let func_type_idx = module.add_func_type(FuncType {
            params: vec![ValType::I32; func.params.len()],
            results: vec![ValType::I32],
        });

        let param_count = func.params.len() as u32;
        let mut ctx = FunctionContext {
            func_index_offset: start_func_idx,
            local_offset: param_count,
            var_map: BTreeMap::new(),
            runtime_alloc: import_count,
            runtime_free: import_count + 4,
            runtime_box_int: import_count + 1,
            runtime_unbox_int: import_count + 2,
            runtime_make_tagged: import_count + 1,
            runtime_get_tag: import_count + 2,
            runtime_get_payload: import_count + 3,
            runtime_nil: import_count + 6,
            runtime_cons: import_count + 7,
            runtime_list_head: import_count + 8,
            runtime_list_tail: import_count + 9,
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
            kind: ExportKind::Func(func_type_idx),
        });
    }

    let wat = crate::emit::wat::emit_wat(&module);
    GleamUnisonAdapter {
        module,
        wat,
        import_count,
    }
}
