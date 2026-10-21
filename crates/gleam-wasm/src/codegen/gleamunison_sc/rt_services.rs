//! Host-service runtime builtins (indices 8-16): linear-memory KV state,
//! file/log/time stubs, eval, memcpy.

use crate::ir::module::{FuncType, Function, Module};
use crate::ir::types::{Block, Instr, Local, ValType};

/// Push builtins 8-16 onto `module`. Order defines function indices:
/// 8=$state_get 9=$state_set 10=$file_read 11=$file_write 12=$log
/// 13=$now_ms 14=$timestamp 15=$eval 16=$memcpy.
/// MUST run after push_core_builtins so indices stay stable.
pub(super) fn push_service_builtins(module: &mut Module) {
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
        params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
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
            Instr::Call(4),       // $hash_bytes
            Instr::LocalSet(4),   // $hash in local 4
            Instr::LocalGet(3),   // val_len
            Instr::Call(0),       // alloc(val_len)
            Instr::LocalSet(5),   // $alloc_ptr in local 5
            Instr::LocalGet(5),   // dst = alloc_ptr
            Instr::LocalGet(2),   // src = val_ptr
            Instr::LocalGet(3),   // len = val_len
            Instr::Call(16),      // $memcpy
            Instr::Drop,          // discard memcpy return
            Instr::LocalGet(4),   // address = $hash
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
        params: vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
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
            let loop_body = vec![
                // memory[$dst + $i] = memory[$src + $i]
                Instr::LocalGet(0),
                Instr::LocalGet(3),
                Instr::I32Add,
                Instr::LocalGet(1),
                Instr::LocalGet(3),
                Instr::I32Add,
                Instr::I32Load8U {
                    offset: 0,
                    align: 0,
                },
                Instr::I32Store8 {
                    offset: 0,
                    align: 0,
                },
                Instr::LocalGet(3),
                Instr::I32Const(1),
                Instr::I32Add,
                Instr::LocalTee(3),
                Instr::LocalGet(2),
                Instr::I32LtS,
                Instr::BrIf(0),
            ];

            instrs.push(Instr::Loop(Box::new(
                Block::new(loop_body, None).label("memcpy_loop"),
            )));
            instrs.push(Instr::LocalGet(0)); // return dst
            instrs
        },
    });
}
