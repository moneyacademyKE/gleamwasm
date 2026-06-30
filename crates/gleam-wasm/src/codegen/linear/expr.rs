use crate::ir::module::Module;
use crate::ir::types::{Block, Instr, ValType};

use crate::codegen::ast::{BinOp, TypedExpr};

pub struct FunctionContext {
    pub func_index_offset: u32,
    pub local_offset: u32,
    pub var_map: std::collections::BTreeMap<String, u32>,
    pub runtime_alloc: u32,
    pub runtime_free: u32,
    pub runtime_box_int: u32,
    pub runtime_unbox_int: u32,
    pub runtime_make_tagged: u32,
    pub runtime_get_tag: u32,
    pub runtime_get_payload: u32,
    pub runtime_nil: u32,
    pub runtime_cons: u32,
    pub runtime_list_head: u32,
    pub runtime_list_tail: u32,
}

impl FunctionContext {
    /// i32 temp register 0
    fn t0(&self) -> u32 { self.local_offset }
    /// i32 temp register 1
    fn t1(&self) -> u32 { self.local_offset + 1 }
    /// i32 temp register 2
    fn t2(&self) -> u32 { self.local_offset + 2 }
    /// f64 temp register 0
    fn ft0(&self) -> u32 { self.local_offset + 4 }
}

/// Compile a single TypedExpr to linear memory instructions.
/// All values are i32 tagged references in linear memory.
/// Returns a Vec<Instr> that leaves a raw i32 on the stack.
pub fn compile_linear_expr(
    expr: &TypedExpr,
    _module: &Module,
    ctx: &FunctionContext,
) -> Vec<Instr> {
    let mut body = Vec::new();
    compile_recursive(&mut body, expr, ctx);
    body
}

fn compile_recursive(body: &mut Vec<Instr>, expr: &TypedExpr, ctx: &FunctionContext) {
    match expr {
        TypedExpr::Int(i) => {
            body.push(Instr::I32Const(*i as i32));
        }
        TypedExpr::Float(f) => {
            body.push(Instr::I32Const(8));
            body.push(Instr::Call(ctx.runtime_alloc));
            body.push(Instr::LocalTee(ctx.ft0())); // save ptr in f64 temp slot
            body.push(Instr::F64Const(*f));
            body.push(Instr::F64Store { offset: 0, align: 3 });
            body.push(Instr::I32Const(1)); // tag 1 = Float
            body.push(Instr::LocalGet(ctx.ft0())); // payload = heap ptr
            body.push(Instr::Call(ctx.runtime_make_tagged));
        }
        TypedExpr::Bool(b) => {
            body.push(Instr::I32Const(2)); // tag 2 = Bool
            body.push(Instr::I32Const(if *b { 1 } else { 0 }));
            body.push(Instr::Call(ctx.runtime_make_tagged));
        }
        TypedExpr::Nil => {
            body.push(Instr::I32Const(3)); // tag 3 = Nil
            body.push(Instr::I32Const(0));
            body.push(Instr::Call(ctx.runtime_make_tagged));
        }
        TypedExpr::ListNil => {
            body.push(Instr::Call(ctx.runtime_nil));
        }
        TypedExpr::ListCons {
            head, tail, type_: _,
        } => {
            compile_recursive(body, head, ctx);
            compile_recursive(body, tail, ctx);
            body.push(Instr::Call(ctx.runtime_cons));
        }
        TypedExpr::Var { name, type_: _ } => {
            let idx = ctx.var_map.get(name).copied().unwrap_or(0);
            body.push(Instr::LocalGet(idx));
        }
        TypedExpr::BinOp {
            op,
            left,
            right,
            type_,
        } => {
            compile_recursive(body, left, ctx);
            compile_recursive(body, right, ctx);
            match type_ {
                ValType::I64 => {
                    // Stack: [left, right] — pop right into t1, left into t0
                    body.push(Instr::LocalSet(ctx.t1())); // pop right
                    body.push(Instr::LocalSet(ctx.t0())); // pop left
                    body.push(Instr::LocalGet(ctx.t0()));
                    body.push(Instr::LocalGet(ctx.t1()));
                    emit_binop_i32(body, op);
                }
                ValType::F64 => {
                    emit_unbox_float(body, ctx);
                    body.push(Instr::LocalSet(ctx.ft0())); // left f64
                    emit_unbox_float(body, ctx);
                    body.push(Instr::LocalGet(ctx.ft0()));
                    emit_binop_f64(body, op);
                    body.push(Instr::I32Const(8));
                    body.push(Instr::Call(ctx.runtime_alloc));
                    body.push(Instr::LocalTee(ctx.t0())); // save ptr
                    body.push(Instr::F64Store { offset: 0, align: 3 });
                    body.push(Instr::I32Const(1));
                    body.push(Instr::LocalGet(ctx.t0()));
                    body.push(Instr::Call(ctx.runtime_make_tagged));
                }
                _ => {
                    body.push(Instr::Unreachable);
                }
            }
        }
        TypedExpr::If {
            cond,
            then_branch,
            else_branch,
            type_: _,
        } => {
            compile_recursive(body, cond, ctx);
            // Result is already raw i32 — compare with 0 (0 = false)
            body.push(Instr::I32Const(0));
            body.push(Instr::I32Ne);

            let mut then_body = Vec::new();
            compile_recursive(&mut then_body, then_branch, ctx);

            let else_body = if let TypedExpr::Nil = **else_branch {
                vec![
                    Instr::I32Const(3),
                    Instr::I32Const(0),
                    Instr::Call(ctx.runtime_make_tagged),
                ]
            } else {
                let mut eb = Vec::new();
                compile_recursive(&mut eb, else_branch, ctx);
                eb
            };

            let then_block = Block::new(then_body, Some(ValType::I32)).label("then");
            let else_block = Block::new(else_body, Some(ValType::I32)).label("else");
            body.push(Instr::If {
                then_branch: Box::new(then_block),
                else_branch: Some(Box::new(else_block)),
            });
        }
        TypedExpr::Let {
            name: _,
            value,
            body: inner_body,
            type_: _,
        } => {
            compile_recursive(body, value, ctx);
            body.push(Instr::LocalSet(ctx.t2())); // let binding in temp
            compile_recursive(body, inner_body, ctx);
        }
        TypedExpr::Call { .. } => {
            body.push(Instr::Unreachable); // not supported in linear target yet
        }
        TypedExpr::TailCall {
            args,
            type_: _,
            ..
        } => {
            for arg in args {
                compile_recursive(body, arg, ctx);
            }
            body.push(Instr::ReturnCall(ctx.func_index_offset));
        }
        TypedExpr::Match {
            scrutinee,
            cases,
            type_: _,
        } => {
            compile_linear_match(body, scrutinee, cases, ctx);
        }
        _ => {
            body.push(Instr::Unreachable);
        }
    }
}

fn compile_linear_match(
    body: &mut Vec<Instr>,
    scrutinee: &TypedExpr,
    cases: &[crate::codegen::ast::MatchCase],
    ctx: &FunctionContext,
) {
    if cases.is_empty() {
        body.push(Instr::Unreachable);
        return;
    }

    compile_recursive(body, scrutinee, ctx);
    // The scrutinee is an 8-byte tagged value — dup to t2 before extracting tag,
    // then free the original after we're done.
    body.push(Instr::LocalTee(ctx.t1())); // save scrutinee pointer in t1
    body.push(Instr::Call(ctx.runtime_get_tag));
    body.push(Instr::LocalSet(ctx.t0())); // store tag in temp 0

    let num_cases = cases.len();

    for (i, case) in cases.iter().enumerate() {
        let is_last = i == num_cases - 1;

        body.push(Instr::LocalGet(ctx.t0()));
        body.push(Instr::I32Const(case.variant_index as i32));
        body.push(Instr::I32Eq);

        let mut then_body = Vec::new();
        compile_recursive(&mut then_body, &case.body, ctx);

        let then_block =
            Block::new(then_body, Some(ValType::I32)).label(format!("case_{}", case.variant_index));

        if is_last {
            let else_block = Block::new(vec![Instr::Unreachable], None).label("no_match");
            body.push(Instr::If {
                then_branch: Box::new(then_block),
                else_branch: Some(Box::new(else_block)),
            });
        } else {
            body.push(Instr::If {
                then_branch: Box::new(then_block),
                else_branch: None,
            });
        }
    }
}

fn emit_unbox_float(body: &mut Vec<Instr>, ctx: &FunctionContext) {
    body.push(Instr::Call(ctx.runtime_get_payload));
    body.push(Instr::F64Load { offset: 0, align: 3 });
}

fn emit_binop_i32(body: &mut Vec<Instr>, op: &BinOp) {
    match op {
        BinOp::Add => body.push(Instr::I32Add),
        BinOp::Sub => body.push(Instr::I32Sub),
        BinOp::Mul => body.push(Instr::I32Mul),
        BinOp::Div => body.push(Instr::I32DivS),
        BinOp::Eq => body.push(Instr::I32Eq),
        BinOp::Ne => body.push(Instr::I32Ne),
        BinOp::Lt => body.push(Instr::I32LtS),
        BinOp::Gt => body.push(Instr::I32GtS),
        BinOp::Le => body.push(Instr::I32LeS),
        BinOp::Ge => body.push(Instr::I32GeS),
    }
}

fn emit_binop_f64(body: &mut Vec<Instr>, op: &BinOp) {
    match op {
        BinOp::Add => body.push(Instr::F64Add),
        BinOp::Sub => body.push(Instr::F64Sub),
        BinOp::Mul => body.push(Instr::F64Mul),
        BinOp::Div => body.push(Instr::F64Div),
        BinOp::Eq => body.push(Instr::F64Eq),
        BinOp::Ne => body.push(Instr::F64Ne),
        BinOp::Lt => body.push(Instr::F64Lt),
        BinOp::Gt => body.push(Instr::F64Gt),
        BinOp::Le => body.push(Instr::F64Le),
        BinOp::Ge => body.push(Instr::F64Ge),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_ctx() -> FunctionContext {
        FunctionContext {
            func_index_offset: 6,
            local_offset: 2,
            var_map: BTreeMap::new(),
            runtime_alloc: 0,
            runtime_free: 1,
            runtime_box_int: 2,
            runtime_unbox_int: 3,
            runtime_make_tagged: 2,
            runtime_get_tag: 3,
            runtime_get_payload: 4,
            runtime_nil: 9,
            runtime_cons: 10,
            runtime_list_head: 11,
            runtime_list_tail: 12,
        }
    }

    #[test]
    fn test_linear_int_literal() {
        let expr = TypedExpr::Int(42);
        let module = Module::new();
        let body = compile_linear_expr(&expr, &module, &test_ctx());
        assert_eq!(body[0], Instr::I32Const(42));
    }

    #[test]
    fn test_linear_bool_true() {
        let expr = TypedExpr::Bool(true);
        let module = Module::new();
        let body = compile_linear_expr(&expr, &module, &test_ctx());
        assert_eq!(body[0], Instr::I32Const(2));
        assert_eq!(body[1], Instr::I32Const(1));
        assert_eq!(body[2], Instr::Call(2)); // make_tagged at index 2
    }

    #[test]
    fn test_linear_int_addition() {
        let expr = TypedExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(TypedExpr::Int(10)),
            right: Box::new(TypedExpr::Int(20)),
            type_: ValType::I64,
        };
        let module = Module::new();
        let body = compile_linear_expr(&expr, &module, &test_ctx());
        assert!(body.contains(&Instr::I32Add));
    }

    #[test]
    fn test_linear_nil() {
        let expr = TypedExpr::Nil;
        let module = Module::new();
        let body = compile_linear_expr(&expr, &module, &test_ctx());
        assert_eq!(body[0], Instr::I32Const(3)); // tag 3 = Nil
        assert_eq!(body[2], Instr::Call(2)); // make_tagged at index 2
    }

    #[test]
    fn test_linear_match_two_variants() {
        let expr = TypedExpr::Match {
            scrutinee: Box::new(TypedExpr::Int(0)),
            cases: vec![
                crate::codegen::ast::MatchCase {
                    variant_index: 1,
                    bindings: vec![],
                    body: Box::new(TypedExpr::Int(10)),
                },
                crate::codegen::ast::MatchCase {
                    variant_index: 2,
                    bindings: vec![],
                    body: Box::new(TypedExpr::Int(20)),
                },
            ],
            type_: ValType::I64,
        };
        let module = Module::new();
        let body = compile_linear_expr(&expr, &module, &test_ctx());
        assert!(body.contains(&Instr::Call(3))); // get_tag at index 3
        assert!(body.contains(&Instr::I32Eq));
    }
}
