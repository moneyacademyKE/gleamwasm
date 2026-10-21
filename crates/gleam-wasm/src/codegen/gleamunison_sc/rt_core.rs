//! Core runtime builtins (indices 0-7): bump allocator, tagged-cell
//! primitives, FNV-1a hashing.

use crate::ir::module::{FuncType, Function, Module};
use crate::ir::types::{Block, Instr, Local, ValType};

/// Push builtins 0-7 onto `module`. Order defines function indices:
/// 0=$alloc 1=$make_tagged 2=$get_tag 3=$get_payload 4=$hash_bytes
/// 5=$hex_to_bytes 6=$hash_equal 7=$hash_to_hex.
pub(super) fn push_core_builtins(module: &mut Module) {
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
        body: vec![
            Instr::LocalGet(0),
            Instr::I32Load {
                offset: 0,
                align: 2,
            },
        ],
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
        body: vec![
            Instr::LocalGet(0),
            Instr::I32Load {
                offset: 4,
                align: 2,
            },
        ],
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
            let loop_body = vec![
                // $byte = load8u($data + $i)
                Instr::LocalGet(0),
                Instr::LocalGet(3),
                Instr::I32Add,
                Instr::I32Load8U {
                    offset: 0,
                    align: 0,
                },
                Instr::LocalSet(4),
                // $hash = $hash ^ $byte
                Instr::LocalGet(2),
                Instr::LocalGet(4),
                Instr::I32Xor,
                Instr::LocalSet(2),
                // $hash = $hash * 0x01000193
                Instr::LocalGet(2),
                Instr::I32Const(0x0193),
                Instr::I32Const(0x0100),
                Instr::I32Const(16),
                Instr::I32Shl,
                Instr::I32Or,
                Instr::I32Mul,
                Instr::LocalSet(2),
                // $i = $i + 1
                Instr::LocalGet(3),
                Instr::I32Const(1),
                Instr::I32Add,
                Instr::LocalSet(3),
                // br_if $loop ($i < $len)
                Instr::LocalGet(3),
                Instr::LocalGet(1),
                Instr::I32LtS,
                Instr::BrIf(0),
            ];

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
                Instr::Block(Box::new(
                    Block::new(
                        vec![
                            Instr::LocalGet(1),
                            Instr::LocalGet(3),
                            Instr::I32Eq,
                            Instr::BrIf(0),
                            Instr::I32Const(0),
                            Instr::Return,
                        ],
                        None,
                    )
                    .label("check_len"),
                )),
                // $i = 0
                Instr::I32Const(0),
                Instr::LocalSet(4),
            ];
            let loop_body = vec![
                Instr::LocalGet(0),
                Instr::LocalGet(4),
                Instr::I32Add,
                Instr::I32Load8U {
                    offset: 0,
                    align: 0,
                },
                Instr::LocalGet(2),
                Instr::LocalGet(4),
                Instr::I32Add,
                Instr::I32Load8U {
                    offset: 0,
                    align: 0,
                },
                Instr::I32Ne,
                Instr::BrIf(1), // br to mismatch handler below
                Instr::LocalGet(4),
                Instr::I32Const(1),
                Instr::I32Add,
                Instr::LocalSet(4),
                Instr::LocalGet(4),
                Instr::LocalGet(1),
                Instr::I32LtS,
                Instr::BrIf(0),
            ];

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
}
