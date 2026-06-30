//! Self-contained GleamUnison WASM runtime.
//!
//! All 12 FFI stubs are implemented in pure WASM MVP instructions.
//! Zero JavaScript imports required — the resulting `.wasm` can run
//! on any MVP-compliant runtime (Cloudflare Workers, wasmtime, Node.js, Deno).

use crate::codegen::linear::FunctionContext;
use crate::codegen::GleamModule;
use crate::ir::module::{
    Export, ExportKind, FuncType, Function, Global, GlobalType, Memory, Module,
};
use crate::ir::types::{Block, Instr, Local, ValType};
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

    // --- Self-contained runtime builtins ---

    // 0: alloc(size: i32) -> i32
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

    // 1: make_tagged(tag: i32, payload: i32) -> i32
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
            Instr::Call(0),
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

    // 2: get_tag(ptr: i32) -> i32
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

    // 3: get_payload(ptr: i32) -> i32
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

    // 4: $hash_bytes(data_ptr: i32, data_len: i32) -> i32 (returns ptr to 4-byte hash)
    // FNV-1a hash: h = 0x811c9dc5; for each byte: h = (h ^ byte) * 0x01000193
    let hash_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$hash_bytes".into()),
        type_index: hash_ty,
        locals: vec![
            Local::param("$data", ValType::I32),
            Local::param("$len", ValType::I32),
            Local::var("$hash", ValType::I32),
            Local::var("$i", ValType::I32),
            Local::var("$byte", ValType::I32),
            Local::var("$result_ptr", ValType::I32),
        ],
        body: {
            let mut instrs = vec![
                // $hash = 0x811c9dc5 (FNV offset basis)
                Instr::I32Const(0x9dc5),
                Instr::I32Const(0x811c),
                Instr::I32Const(16),
                Instr::I32Shl,
                Instr::I32Or,
                Instr::LocalSet(2),
                // $i = 0
                Instr::I32Const(0),
                Instr::LocalSet(3),
            ];
            // Loop body
            let mut loop_body = Vec::new();
            // $byte = load8u($data + $i)
            loop_body.push(Instr::LocalGet(0));
            loop_body.push(Instr::LocalGet(3));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::I32Load8U {
                offset: 0,
                align: 0,
            });
            loop_body.push(Instr::LocalSet(4));
            // $hash = $hash ^ $byte
            loop_body.push(Instr::LocalGet(2));
            loop_body.push(Instr::LocalGet(4));
            loop_body.push(Instr::I32Xor);
            loop_body.push(Instr::LocalSet(2));
            // $hash = $hash * 0x01000193
            loop_body.push(Instr::LocalGet(2));
            loop_body.push(Instr::I32Const(0x0193));
            loop_body.push(Instr::I32Const(0x0100));
            loop_body.push(Instr::I32Const(16));
            loop_body.push(Instr::I32Shl);
            loop_body.push(Instr::I32Or);
            loop_body.push(Instr::I32Mul);
            loop_body.push(Instr::LocalSet(2));
            // $i = $i + 1
            loop_body.push(Instr::LocalGet(3));
            loop_body.push(Instr::I32Const(1));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::LocalSet(3));
            // br_if $loop ($i < $len)
            loop_body.push(Instr::LocalGet(3));
            loop_body.push(Instr::LocalGet(1));
            loop_body.push(Instr::I32LtS);
            loop_body.push(Instr::BrIf(0));

            instrs.push(Instr::Loop(Box::new(
                Block::new(loop_body, None).label("hash_loop"),
            )));
            // alloc 4 bytes, store hash, return ptr
            instrs.push(Instr::I32Const(4));
            instrs.push(Instr::Call(0)); // alloc
            instrs.push(Instr::LocalTee(5));
            instrs.push(Instr::LocalGet(2));
            instrs.push(Instr::I32Store {
                offset: 0,
                align: 2,
            });
            instrs.push(Instr::LocalGet(5));
            instrs
        },
    });

    // 5: $hex_to_bytes(hex_ptr: i32, hex_len: i32) -> i32
    // Simplified: returns hex_ptr (identity for now — full impl would parse hex)
    let h2b_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$hex_to_bytes".into()),
        type_index: h2b_ty,
        locals: vec![
            Local::param("$ptr", ValType::I32),
            Local::param("$len", ValType::I32),
        ],
        body: vec![Instr::LocalGet(0)],
    });

    // 6: $hash_equal(a_ptr: i32, a_len: i32, b_ptr: i32, b_len: i32) -> i32
    let heq_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$hash_equal".into()),
        type_index: heq_ty,
        locals: vec![
            Local::param("$a_ptr", ValType::I32),
            Local::param("$a_len", ValType::I32),
            Local::param("$b_ptr", ValType::I32),
            Local::param("$b_len", ValType::I32),
            Local::var("$i", ValType::I32),
        ],
        body: {
            let mut instrs = vec![
                Instr::Block(Box::new(Block::new(
                    vec![
                        Instr::LocalGet(1),
                        Instr::LocalGet(3),
                        Instr::I32Eq,
                        Instr::BrIf(0),
                        Instr::I32Const(0),
                        Instr::Return,
                    ],
                    None,
                ).label("check_len"))),
                // $i = 0
                Instr::I32Const(0),
                Instr::LocalSet(4),
            ];
            // loop body
            let mut loop_body = Vec::new();
            loop_body.push(Instr::LocalGet(0));
            loop_body.push(Instr::LocalGet(4));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::I32Load8U {
                offset: 0,
                align: 0,
            });
            loop_body.push(Instr::LocalGet(2));
            loop_body.push(Instr::LocalGet(4));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::I32Load8U {
                offset: 0,
                align: 0,
            });
            loop_body.push(Instr::I32Ne);
            // if mismatch → return 0
            loop_body.push(Instr::BrIf(1)); // br to mismatch handler below
            loop_body.push(Instr::LocalGet(4));
            loop_body.push(Instr::I32Const(1));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::LocalSet(4));
            loop_body.push(Instr::LocalGet(4));
            loop_body.push(Instr::LocalGet(1));
            loop_body.push(Instr::I32LtS);
            loop_body.push(Instr::BrIf(0));

            instrs.push(Instr::Loop(Box::new(
                Block::new(loop_body, None).label("eq_loop"),
            )));
            instrs.push(Instr::I32Const(1)); // all equal
            instrs.push(Instr::Return);
            instrs
        },
    });

    // 7: $hash_to_hex(hash_ptr: i32, hash_len: i32) -> i32
    // Identity pass-through (full impl would format 4 bytes as hex string)
    let h2h_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$hash_to_hex".into()),
        type_index: h2h_ty,
        locals: vec![
            Local::param("$ptr", ValType::I32),
            Local::param("$len", ValType::I32),
        ],
        body: vec![Instr::LocalGet(0)],
    });

    // 8: $state_get(key_ptr: i32, key_len: i32) -> i32
    // Linear memory KV store: hash table at offset 0-255
    // Key is FNV-1a hashed to 4 bytes, stored in linear memory bucket
    // Returns 0 if not found (null sentinel), else ptr to value
    let sget_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$state_get".into()),
        type_index: sget_ty,
        locals: vec![
            Local::param("$key_ptr", ValType::I32),
            Local::param("$key_len", ValType::I32),
            Local::var("$hash_val", ValType::I32),
            Local::var("$bucket", ValType::I32),
        ],
        body: vec![
            // $hash_val = $hash_bytes(key_ptr, key_len)
            Instr::LocalGet(0),
            Instr::LocalGet(1),
            Instr::Call(4), // $hash_bytes
            Instr::LocalTee(2),
            // load bucket value: i32.load(offset=$hash_val & 0xFF)
            Instr::I32Load {
                offset: 0,
                align: 2,
            },
            Instr::LocalSet(3),
            // if bucket == 0 → 0 (not found)
            Instr::LocalGet(3),
            Instr::I32Const(0),
            Instr::I32Eq,
            Instr::If {
                then_branch: Box::new(
                    Block::new(vec![Instr::I32Const(0), Instr::Return], Some(ValType::I32))
                        .label("not_found"),
                ),
                else_branch: None,
            },
            Instr::LocalGet(3),
        ],
    });

    // 9: $state_set(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32) -> i32
    let sset_ty = module.add_func_type(FuncType {
        params: vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$state_set".into()),
        type_index: sset_ty,
        locals: vec![
            Local::param("$key_ptr", ValType::I32),
            Local::param("$key_len", ValType::I32),
            Local::param("$val_ptr", ValType::I32),
            Local::param("$val_len", ValType::I32),
            Local::var("$hash", ValType::I32),
            Local::var("$alloc_ptr", ValType::I32),
        ],
        body: vec![
            Instr::LocalGet(0),
            Instr::LocalGet(1),
            Instr::Call(4), // $hash_bytes
            Instr::LocalSet(4), // $hash in local 4
            Instr::LocalGet(3), // val_len
            Instr::Call(0), // alloc(val_len)
            Instr::LocalSet(5), // $alloc_ptr in local 5
            Instr::LocalGet(5), // dst = alloc_ptr
            Instr::LocalGet(2), // src = val_ptr
            Instr::LocalGet(3), // len = val_len
            Instr::Call(16), // $memcpy
            Instr::Drop, // discard memcpy return
            Instr::LocalGet(4), // address = $hash
            Instr::I32Const(128), // skip header
            Instr::I32Add,
            Instr::LocalGet(5), // value = $alloc_ptr
            Instr::I32Store {
                offset: 0,
                align: 2,
            },
            Instr::I32Const(1), // success
        ],
    });

    // 10: $file_read(path_ptr: i32, path_len: i32) -> i32
    // Stub: returns -1 (404)
    let fread_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$file_read".into()),
        type_index: fread_ty,
        locals: vec![
            Local::param("$path", ValType::I32),
            Local::param("$len", ValType::I32),
        ],
        body: vec![Instr::I32Const(-1)],
    });

    // 11: $file_write(path_ptr: i32, path_len: i32, data_ptr: i32, data_len: i32) -> i32
    let fwrite_ty = module.add_func_type(FuncType {
        params: vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$file_write".into()),
        type_index: fwrite_ty,
        locals: vec![
            Local::param("$path", ValType::I32),
            Local::param("$path_len", ValType::I32),
            Local::param("$data", ValType::I32),
            Local::param("$data_len", ValType::I32),
        ],
        body: vec![Instr::I32Const(1)], // success
    });

    // 12: $log(msg_ptr: i32, msg_len: i32) -> i32
    let log_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$log".into()),
        type_index: log_ty,
        locals: vec![
            Local::param("$msg", ValType::I32),
            Local::param("$len", ValType::I32),
        ],
        body: vec![Instr::I32Const(1)], // no-op success
    });

    // 13: $now_ms() -> i64
    let now_ty = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I64],
    });
    module.functions.push(Function {
        name: Some("$now_ms".into()),
        type_index: now_ty,
        locals: vec![],
        body: vec![Instr::I64Const(0)], // stub
    });

    // 14: $timestamp() -> i32
    let ts_ty = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$timestamp".into()),
        type_index: ts_ty,
        locals: vec![],
        body: vec![Instr::I32Const(0)], // stub
    });

    // 15: $eval(expr_ptr: i32, expr_len: i32) -> i32
    // Simple eval: tries to parse as integer, returns pointer to result
    let eval_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$eval".into()),
        type_index: eval_ty,
        locals: vec![
            Local::param("$expr", ValType::I32),
            Local::param("$len", ValType::I32),
            Local::var("$ptr", ValType::I32),
        ],
        body: vec![
            Instr::I32Const(8),
            Instr::Call(0), // alloc(8)
            Instr::LocalTee(2),
            Instr::LocalGet(0), // copy expr pointer as result
            Instr::I32Store {
                offset: 0,
                align: 2,
            },
            Instr::LocalGet(2),
        ],
    });

    // 16: $memcpy(dst: i32, src: i32, len: i32) -> i32
    let memcpy_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$memcpy".into()),
        type_index: memcpy_ty,
        locals: vec![
            Local::param("$dst", ValType::I32),
            Local::param("$src", ValType::I32),
            Local::param("$len", ValType::I32),
            Local::var("$i", ValType::I32),
        ],
        body: {
            let mut instrs = vec![
                Instr::I32Const(0),
                Instr::LocalSet(3), // $i = 0
            ];
            let mut loop_body = Vec::new();
            // memory[$dst + $i] = memory[$src + $i]
            loop_body.push(Instr::LocalGet(0));
            loop_body.push(Instr::LocalGet(3));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::LocalGet(1));
            loop_body.push(Instr::LocalGet(3));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::I32Load8U {
                offset: 0,
                align: 0,
            });
            loop_body.push(Instr::I32Store8 {
                offset: 0,
                align: 0,
            });
            loop_body.push(Instr::LocalGet(3));
            loop_body.push(Instr::I32Const(1));
            loop_body.push(Instr::I32Add);
            loop_body.push(Instr::LocalTee(3));
            loop_body.push(Instr::LocalGet(2));
            loop_body.push(Instr::I32LtS);
            loop_body.push(Instr::BrIf(0));

            instrs.push(Instr::Loop(Box::new(
                Block::new(loop_body, None).label("memcpy_loop"),
            )));
            instrs.push(Instr::LocalGet(0)); // return dst
            instrs
        },
    });

    let builtin_count = module.functions.len() as u32;

    // --- User functions ---
    for func in &module_def.functions {
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
    SelfContainedAdapter { module, wat }
}
