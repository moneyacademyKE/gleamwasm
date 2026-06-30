use std::collections::BTreeMap;

use crate::codegen::GleamModule;
use crate::ir::module::{
    Export, ExportKind, FuncType, Function, Global, GlobalType, Memory, Module,
};
use crate::ir::types::{Block, Instr, Local, ValType};

mod expr;

pub use expr::{compile_linear_expr, FunctionContext};

/// Build the linear memory runtime plus user functions.
/// Returns (module, wat).
pub fn compile_to_linear(module_def: &GleamModule) -> (Module, String) {
    let mut module = Module::new();
    module.memories.push(Memory {
        min: 1,
        max: Some(256),
    });

    // Mutable i32 global for heap pointer (global 0)
    module.globals.push(Global {
        type_: GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        init: vec![Instr::I32Const(8)],
    });

    // Mutable i32 globals for free lists — one head per common allocation size.
    // global 1: free list for 8-byte blocks (tagged values)
    // global 2: free list for 12-byte blocks (cons cells)
    module.globals.push(Global {
        type_: GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        init: vec![Instr::I32Const(0)],
    });
    module.globals.push(Global {
        type_: GlobalType {
            val_type: ValType::I32,
            mutable: true,
        },
        init: vec![Instr::I32Const(0)],
    });

    // --- Runtime builtins ---

    // 0: alloc(size: i32) -> i32
    // Checks free list for exact-size match; bumps heap otherwise.
    let alloc_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    let mut alloc_body: Vec<Instr> = Vec::new();
    // Try free list for 8-byte blocks
    alloc_body.push(Instr::Block(Box::new(Block::new(vec![
        Instr::LocalGet(0),
        Instr::I32Const(8),
        Instr::I32Ne,
        Instr::BrIf(0),
        Instr::GlobalGet(1),
        Instr::LocalTee(2),
        Instr::I32Const(0),
        Instr::I32Eq,
        Instr::BrIf(0),
        Instr::LocalGet(2),
        Instr::I32Load { offset: 0, align: 2 },
        Instr::GlobalSet(1),
        Instr::LocalGet(2),
        Instr::Return,
    ], None))));
    // Try free list for 12-byte blocks
    alloc_body.push(Instr::Block(Box::new(Block::new(vec![
        Instr::LocalGet(0),
        Instr::I32Const(12),
        Instr::I32Ne,
        Instr::BrIf(0),
        Instr::GlobalGet(2),
        Instr::LocalTee(2),
        Instr::I32Const(0),
        Instr::I32Eq,
        Instr::BrIf(0),
        Instr::LocalGet(2),
        Instr::I32Load { offset: 0, align: 2 },
        Instr::GlobalSet(2),
        Instr::LocalGet(2),
        Instr::Return,
    ], None))));
    // Bump allocate
    alloc_body.push(Instr::GlobalGet(0));
    alloc_body.push(Instr::LocalSet(1));
    alloc_body.push(Instr::GlobalGet(0));
    alloc_body.push(Instr::LocalGet(0));
    alloc_body.push(Instr::I32Add);
    alloc_body.push(Instr::GlobalSet(0));
    alloc_body.push(Instr::LocalGet(1));
    module.functions.push(Function {
        name: Some("$alloc".into()),
        type_index: alloc_ty,
        locals: vec![
            Local::param("$size", ValType::I32),
            Local::var("$ptr", ValType::I32),
            Local::var("$head", ValType::I32),
        ],
        body: alloc_body,
    });

    // 1: free(ptr: i32, size: i32)
    let free_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![],
    });
    module.functions.push(Function {
        name: Some("$free".into()),
        type_index: free_ty,
        locals: vec![
            Local::param("$ptr", ValType::I32),
            Local::param("$size", ValType::I32),
        ],
        body: vec![
            // 8-byte blocks
            Instr::LocalGet(1),
            Instr::I32Const(8),
            Instr::I32Eq,
            Instr::If {
                then_branch: Box::new(Block::new(vec![
                    Instr::LocalGet(0),
                    Instr::GlobalGet(1), // old head
                    Instr::I32Store { offset: 0, align: 2 }, // ptr[0] = old head
                    Instr::LocalGet(0),
                    Instr::GlobalSet(1), // free8 = ptr
                ], None)),
                else_branch: None,
            },
            // 12-byte blocks
            Instr::LocalGet(1),
            Instr::I32Const(12),
            Instr::I32Eq,
            Instr::If {
                then_branch: Box::new(Block::new(vec![
                    Instr::LocalGet(0),
                    Instr::GlobalGet(2), // old head
                    Instr::I32Store { offset: 0, align: 2 }, // ptr[0] = old head
                    Instr::LocalGet(0),
                    Instr::GlobalSet(2), // free12 = ptr
                ], None)),
                else_branch: None,
            },
        ],
    });

    // 2: make_tagged(tag: i32, payload: i32) -> i32
    let make_tagged_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$make_tagged".into()),
        type_index: make_tagged_ty,
        locals: vec![
            Local::param("$tag", ValType::I32),
            Local::param("$payload", ValType::I32),
            Local::var("$ret", ValType::I32),
        ],
        body: vec![
            Instr::I32Const(8),
            Instr::Call(0), // alloc
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
    let get_tag_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$get_tag".into()),
        type_index: get_tag_ty,
        locals: vec![Local::param("$ptr", ValType::I32)],
        body: vec![Instr::LocalGet(0), Instr::I32Load {
            offset: 0,
            align: 2,
        }],
    });

    // 3: get_payload(ptr: i32) -> i32
    let get_payload_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$get_payload".into()),
        type_index: get_payload_ty,
        locals: vec![Local::param("$ptr", ValType::I32)],
        body: vec![Instr::LocalGet(0), Instr::I32Load {
            offset: 4,
            align: 2,
        }],
    });

    // --- List runtime builtins ---
    // Runtime funcs so far:
    //   0: $alloc
    //   1: $free
    //   2: $make_tagged
    //   3: $get_tag
    //   4: $get_payload
    //   5: $nil
    //   6: $cons

    // 5: nil() -> i32  — returns tagged nil pointer
    let nil_ty = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$nil".into()),
        type_index: nil_ty,
        locals: vec![],
        body: vec![
            Instr::I32Const(3), // Nil tag
            Instr::I32Const(0),
            Instr::Call(2), // make_tagged
        ],
    });

    // 5: cons(head: i32, tail: i32) -> i32
    // Allocates a Cons cell with tag 4, stores [tag:4, head, tail] (12 bytes)
    let cons_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32, ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$cons".into()),
        type_index: cons_ty,
        locals: vec![
            Local::param("$head", ValType::I32),
            Local::param("$tail", ValType::I32),
            Local::var("$ptr", ValType::I32),
        ],
        body: vec![
            Instr::I32Const(12),
            Instr::Call(0), // alloc(12)
            Instr::LocalTee(2), // $ptr
            Instr::I32Const(4), // Cons tag
            Instr::I32Store { offset: 0, align: 2 },
            Instr::LocalGet(2),
            Instr::LocalGet(0), // head
            Instr::I32Store { offset: 4, align: 2 },
            Instr::LocalGet(2),
            Instr::LocalGet(1), // tail
            Instr::I32Store { offset: 8, align: 2 },
            Instr::LocalGet(2), // return ptr
        ],
    });

    // 6: list_get_head(cons_ptr: i32) -> i32
    let list_head_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$list_get_head".into()),
        type_index: list_head_ty,
        locals: vec![Local::param("$ptr", ValType::I32)],
        body: vec![
            Instr::LocalGet(0),
            Instr::I32Load { offset: 4, align: 2 },
        ],
    });

    // 7: list_get_tail(cons_ptr: i32) -> i32
    let list_tail_ty = module.add_func_type(FuncType {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    });
    module.functions.push(Function {
        name: Some("$list_get_tail".into()),
        type_index: list_tail_ty,
        locals: vec![Local::param("$ptr", ValType::I32)],
        body: vec![
            Instr::LocalGet(0),
            Instr::I32Load { offset: 8, align: 2 },
        ],
    });

    // --- User functions ---
    let start_func_index = module.functions.len() as u32;

    for func in &module_def.functions {
        let func_type_idx = module.add_func_type(FuncType {
            params: vec![ValType::I32; func.params.len()],
            results: vec![ValType::I32],
        });

        let param_count = func.params.len() as u32;
        let mut ctx = FunctionContext {
            func_index_offset: start_func_index,
            local_offset: param_count,
            var_map: BTreeMap::new(),
            runtime_alloc: 0,
            runtime_free: 1,
            runtime_box_int: 2,
            runtime_unbox_int: 3,
            runtime_make_tagged: 2,
            runtime_get_tag: 3,
            runtime_get_payload: 4,
            runtime_nil: 5,
            runtime_cons: 6,
            runtime_list_head: 7,
            runtime_list_tail: 8,
        };
        for (i, (name, _)) in func.params.iter().enumerate() {
            ctx.var_map.insert(name.clone(), i as u32);
        }

        let compiled = compile_linear_expr(&func.body, &module, &ctx);
        let body = compiled;

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
            body,
        });

        module.exports.push(Export {
            name: func.name.clone(),
            kind: ExportKind::Func(func_type_idx),
        });
    }

    let wat = crate::emit::wat::emit_wat(&module);
    (module, wat)
}
